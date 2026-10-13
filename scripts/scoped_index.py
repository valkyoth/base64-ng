"""Explicitly authorized unsigned scoped indexing, or read-only index verification."""

import argparse
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tarfile
import tempfile

import scoped_candidate as metadata
import scoped_finalizer as scoped
from scoped_evidence import (MAX_BYTES, MAX_ENTRIES, MAX_MANIFEST, SHA256, canonical, digest_stream,
                             read_regular, regular_file, strict_json)


INTEGRATION_ENV = "BASE64_NG_REVIEWED_SCOPED_FINALIZER_COMMIT"
RECORD_ENV = "BASE64_NG_REVIEWED_CANDIDATE_RECORD_SHA256"
WRITE_ENV = "BASE64_NG_FINALIZE_SCOPED_COMMIT"
FINAL = "FINAL-MANIFEST.txt"
INDEX = "SCOPED-INDEX.json"
OUTPUTS = {FINAL, FINAL + ".sig", INDEX}
REQUIRED_CHECKS = {"rust-family-packages", "wasm-npm-package-and-provenance",
                   "sbom", "reproducibility", "exact-candidate-full-release-ci"}


def inventory_tree(root, excluded):
    root = root.absolute()
    for path in (root, *root.parents):
        if not stat.S_ISDIR(path.lstat().st_mode):
            raise ValueError("evidence root has a linked or non-directory ancestor")
    if root.stat().st_mode & 0o077:
        raise ValueError("scoped evidence root must be private (mode 0700)")
    files, directories, pending, total = {}, [], [root], 0
    count = 0
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                count += 1
                if count > MAX_ENTRIES:
                    raise ValueError("evidence inventory exceeds entry limit")
                path = Path(entry.path)
                name = path.relative_to(root).as_posix()
                canonical(name)
                if not re.fullmatch(r"[A-Za-z0-9._/-]+", name):
                    raise ValueError("evidence path is not representable in the signed hash index")
                info = entry.stat(follow_symlinks=False)
                if stat.S_ISDIR(info.st_mode):
                    directories.append(name)
                    pending.append(path)
                elif stat.S_ISREG(info.st_mode) and info.st_nlink == 1:
                    if name not in excluded:
                        with regular_file(path) as stream:
                            pin = digest_stream(stream, MAX_BYTES)
                        total += pin["bytes"]
                        if total > MAX_BYTES:
                            raise ValueError("evidence inventory exceeds aggregate byte limit")
                        files[name] = pin
                else:
                    raise ValueError("linked or special evidence entry: " + name)
    return {"files": dict(sorted(files.items())), "directories": sorted(directories)}


def approval(evidence, candidate, integration, env):
    expected = env.get(RECORD_ENV, "")
    if not SHA256.fullmatch(expected):
        raise ValueError("independent candidate-record SHA-256 is required")
    raw = read_regular(evidence / metadata.REVIEW)
    pin = scoped.record(raw)
    if pin["sha256"] != expected:
        raise ValueError("reviewed candidate record digest mismatch")
    value = strict_json(raw)
    metadata.require_schema_v1(value, metadata.REVIEW)
    if (value["candidate"] != candidate
            or value["integration_commit"] != integration
            or value["status"] != "approved-for-unsigned-index"
            or value["release_authorized"] is not False
            or value["checks"] != dict.fromkeys(sorted(REQUIRED_CHECKS), "passed")):
        raise ValueError("candidate record identity, authority or required checks mismatch")
    return value, pin


def render(root, evidence, env):
    integration = env.get(INTEGRATION_ENV, "")
    scoped.campaign.commit_id(integration)
    for name in ("BASE64_NG_REUSE_EVIDENCE_FROM", "BASE64_NG_CAMPAIGN_SOURCE_COMMIT",
                 "BASE64_NG_ALLOW_DIRTY_EVIDENCE"):
        if env.get(name):
            raise ValueError("scoped indexing cannot combine legacy or dirty-evidence modes")
    candidate = {"commit": scoped.clean_head(root),
                 "tree": scoped.campaign.git(root, "rev-parse", "HEAD^{tree}").decode().strip()}
    approved, review_pin = approval(evidence, candidate, integration, env)
    inputs = inventory_tree(evidence, OUTPUTS | {metadata.REVIEW})
    if inputs != approved["inventory"]:
        raise ValueError("independently reviewed complete candidate inventory mismatch")
    reviews = evidence / "scoped/reviews"
    paths = {"historical": evidence / "scoped/historical.tar.gz",
             "recapture": evidence / "scoped/recapture.tar.gz"}
    prepared = scoped.prepare(root, reviews, paths, integration=integration)
    if prepared["candidate"] != candidate:
        raise ValueError("candidate changed during scoped validation")
    metadata.verify_metadata(root, evidence, candidate, inputs, reviews)
    # The independent digest authenticates the reviewer assertions. Mechanical
    # checks complement that approval; JSON 'passed' fields alone are not trust.
    index = {
        "schema": 1, "evidence_mode": "scoped-claims-v1", "release_authorized": False,
        "candidate": candidate, "reviewed_integration_commit": integration,
        "candidate_review": review_pin, "input_inventory": inputs,
        "scoped_review": prepared,
        "boundary": "unsigned final index; scoped claims retain original sources and exclusions",
    }
    index_raw = (json.dumps(index, indent=2, sort_keys=True) + "\n").encode()
    files = dict(inputs["files"])
    files[metadata.REVIEW] = review_pin
    files[INDEX] = scoped.record(index_raw)
    # Re-read after all validators, including the candidate approval itself.
    if inventory_tree(evidence, OUTPUTS | {metadata.REVIEW}) != inputs \
            or scoped.record(read_regular(evidence / metadata.REVIEW)) != review_pin \
            or scoped.clean_head(root) != candidate["commit"]:
        raise ValueError("evidence or source changed during final indexing")
    lines = ["base64-ng final release evidence index", "", "source:",
             "commit=" + candidate["commit"], "tree_state=clean", "",
             "evidence_mode=scoped-claims-v1", "release_commit=" + candidate["commit"],
             "reviewed_integration_commit=" + integration,
             "scoped_policy_commit=" + scoped.POLICY_COMMIT,
             "release_authorized=false", "", "artifact-hashes:"]
    lines.extend(pin["sha256"] + "  " + metadata.PREFIX + path for path, pin in sorted(files.items()))
    final_raw = ("\n".join(lines) + "\n").encode()
    if len(index_raw) > MAX_MANIFEST or len(final_raw) > MAX_MANIFEST:
        raise ValueError("generated index exceeds metadata byte limit")
    return index_raw, final_raw


def publish_new(path, raw):
    # Link a completely written private temporary file. Unlike replace(), this
    # cannot overwrite an existing destination, including a dangling symlink.
    fd, temporary = tempfile.mkstemp(prefix=".scoped-index-", dir=path.parent)
    linked = False
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(temporary, path)
        linked = True
    finally:
        try:
            os.unlink(temporary)
        except OSError:
            if linked:
                path.unlink()
            raise


def activate(root, evidence, env, write=False):
    if write:
        candidate = scoped.clean_head(root)
        if env.get(WRITE_ENV) != candidate:
            raise ValueError("explicit unsigned-finalization authorization for exact HEAD is required")
        if any(os.path.lexists(evidence / name) for name in OUTPUTS):
            raise ValueError("refusing to replace an index or existing signature")
    index_raw, final_raw = render(root, evidence, env)
    if not write:
        if read_regular(evidence / INDEX) != index_raw or read_regular(evidence / FINAL) != final_raw:
            raise ValueError("scoped final index is stale or altered")
        return
    created_index = False
    try:
        publish_new(evidence / INDEX, index_raw)
        created_index = True
        if scoped.clean_head(root) != strict_json(index_raw)["candidate"]["commit"]:
            raise ValueError("source changed before index publication")
        publish_new(evidence / FINAL, final_raw)
    except BaseException:
        if created_index:
            (evidence / INDEX).unlink()
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    root = scoped.campaign.ROOT
    try:
        activate(root, root / metadata.PREFIX, os.environ, write=args.write)
    except (ValueError, OSError, KeyError, TypeError, tarfile.TarError, subprocess.TimeoutExpired) as error:
        parser.exit(1, "scoped final index: " + str(error) + "\n")
    print("scoped final index: " + ("wrote unsigned index; no release authorization" if args.write
                                   else "claims, candidate metadata and complete inventory verified"))


if __name__ == "__main__":
    main()
