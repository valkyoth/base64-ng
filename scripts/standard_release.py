"""Standard 2.1 closeout: one checklist, retained claims, and exact-candidate artifacts."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tarfile

import scoped_candidate as metadata
import scoped_finalizer as scoped
import scoped_index as index
from scoped_evidence import MAX_MANIFEST, read_regular, strict_json


CHECKLIST = "RELEASE-CHECKLIST.json"
INDEX = "RELEASE-INDEX.json"
MODE = "standard-checklist-v1"
OUTPUTS = {INDEX, index.FINAL, index.FINAL + ".sig"}
WRITE_ENV = "BASE64_NG_FINALIZE_RELEASE_COMMIT"
CHECKS = {"security-review", "artifact-inspection", "retained-campaigns"}
CLAIM_FIELDS = ("approved_policy", "scoped_claim_approval", "packets", "claims",
                "conditions_preserved", "assembly_boundary")
LEGACY_ENV = (index.INTEGRATION_ENV, index.RECORD_ENV, index.WRITE_ENV,
              "BASE64_NG_REUSE_EVIDENCE_FROM", "BASE64_NG_CAMPAIGN_SOURCE_COMMIT",
              "BASE64_NG_ALLOW_DIRTY_EVIDENCE")


def candidate(root):
    return {"commit": scoped.clean_head(root),
            "tree": scoped.campaign.git(root, "rev-parse", "HEAD^{tree}").decode().strip()}


def assessment(root, current):
    # Retain the already reviewed 2.1 claims and their original source identities.
    # This is not a generic permission to edit campaign policies or runtime code.
    for name in (scoped.campaign.POLICY_PATH, scoped.campaign.ATTESTATIONS_PATH):
        if read_regular(root / name) != scoped.campaign.git(root, "show", scoped.POLICY_COMMIT + ":" + name):
            raise ValueError("retained campaign policy changed: " + name)
    policy = scoped.campaign.load_policy(root)
    base = policy["sources"]["recapture"]
    if scoped.campaign.git(root, "rev-parse", base["commit"] + "^{tree}").decode().strip() != base["tree"]:
        raise ValueError("recapture source tree mismatch")
    scoped.campaign.git(root, "merge-base", "--is-ancestor", base["commit"], current["commit"])
    paths = scoped.campaign.git(root, "diff", "--no-renames", "--name-only", "-z",
                                base["commit"], current["commit"]).decode().split("\0")
    paths = sorted(path for path in paths if path)
    for path in paths:
        documentation = (path.startswith(("docs/", "release-notes/", "security/pentest/"))
                         or path in {"README.md", "CHANGELOG.md", "SECURITY.md"})
        tooling = path.startswith("scripts/") and path.endswith((".py", ".sh", ".ps1"))
        retained_policy = path in (scoped.campaign.POLICY_PATH, scoped.campaign.ATTESTATIONS_PATH)
        if not (documentation or tooling or retained_policy):
            raise ValueError("runtime/build/package input changed; reassess affected campaigns: " + path)
    return policy, {"baseline": base, "paths": paths}


def text(value, label):
    if not isinstance(value, str) or not value.strip():
        raise ValueError("missing " + label)


def checklist(evidence, current, impact):
    value = strict_json(read_regular(evidence / CHECKLIST))
    metadata.require_schema_v1(value, CHECKLIST)
    if (value["candidate"] != current
            or value["checks"] != dict.fromkeys(sorted(CHECKS), "passed")
            or value["change_assessment"]["baseline"] != impact["baseline"]
            or value["change_assessment"]["paths"] != impact["paths"]):
        raise ValueError("checklist source, change assessment or required checks mismatch")
    text(value["reviewer"], "maintainer/reviewer")
    text(value["change_assessment"]["rationale"], "change-impact rationale")
    # The report is an evidence attachment, never another source commit.
    if value["security_review"] != "candidate/security-review.md":
        raise ValueError("unexpected security review path")
    lines = read_regular(evidence / value["security_review"]).decode().splitlines()
    for key, expected in (("Status", "PASS"), ("Reviewed-Commit", current["commit"]),
                          ("Reviewed-Tree", current["tree"])):
        if [line for line in lines if line.startswith(key + ":")] != [key + ": " + expected]:
            raise ValueError("security review does not cover exact candidate: " + key)
    for key in ("Tester", "Scope"):
        matches = [line[len(key) + 2:] for line in lines if line.startswith(key + ": ")]
        if len(matches) != 1:
            raise ValueError("missing or duplicate security review " + key)
        text(matches[0], key)
    return value


def render(root, evidence, env):
    if any(env.get(name) for name in LEGACY_ENV):
        raise ValueError("standard closeout cannot mix legacy approval or dirty-evidence modes")
    current = candidate(root)
    policy, impact = assessment(root, current)
    inputs = index.inventory_tree(evidence, OUTPUTS)
    review = checklist(evidence, current, impact)
    reviews = evidence / "scoped/reviews"
    claims = scoped.prepare_claims(root, reviews, {
        "historical": evidence / "scoped/historical.tar.gz",
        "recapture": evidence / "scoped/recapture.tar.gz",
    }, policy, current)
    metadata.verify_metadata(root, evidence, current, inputs, reviews)
    value = {"schema": 1, "evidence_mode": MODE, "candidate": current,
             "release_authorized": False, "checklist": review,
             "input_inventory": inputs,
             "retained_claims": {key: claims[key] for key in CLAIM_FIELDS},
             "review_model": "one exact-candidate maintainer checklist; prior approval workflow is historical",
             "boundary": "maintainer-reviewed standard release; no high-assurance certification"}
    raw = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
    files = dict(inputs["files"], **{INDEX: scoped.record(raw)})
    lines = ["base64-ng final release evidence index", "", "source:",
             "commit=" + current["commit"], "tree_state=clean", "",
             "evidence_mode=" + MODE, "release_commit=" + current["commit"],
             "release_authorized=false", "", "artifact-hashes:"]
    lines.extend(pin["sha256"] + "  " + metadata.PREFIX + path for path, pin in sorted(files.items()))
    final = ("\n".join(lines) + "\n").encode()
    if len(raw) > MAX_MANIFEST or len(final) > MAX_MANIFEST:
        raise ValueError("generated index exceeds metadata limit")
    if index.inventory_tree(evidence, OUTPUTS) != inputs or candidate(root) != current:
        raise ValueError("source or evidence changed during verification")
    return raw, final


def run(root, evidence, env, mode):
    if any(os.path.lexists(evidence / name) for name in (index.INDEX, "EQUIVALENCE-MANIFEST.txt")):
        raise ValueError("standard closeout cannot mix legacy indexes")
    if mode == "template":
        if any(os.path.lexists(evidence / name) for name in OUTPUTS):
            raise ValueError("refusing to modify a finalized collection")
        current = candidate(root)
        _, impact = assessment(root, current)
        index.inventory_tree(evidence, OUTPUTS)
        value = {"schema": 1, "candidate": current, "reviewer": "",
                 "security_review": "candidate/security-review.md",
                 "checks": dict.fromkeys(sorted(CHECKS), "pending"),
                 "change_assessment": dict(impact, rationale="")}
        index.publish_new(evidence / CHECKLIST, (json.dumps(value, indent=2) + "\n").encode())
        return
    if mode == "write":
        if env.get(WRITE_ENV) != candidate(root)["commit"]:
            raise ValueError("explicit maintainer authorization for exact HEAD is required")
        if any(os.path.lexists(evidence / name) for name in (index.FINAL, index.FINAL + ".sig")):
            raise ValueError("refusing to replace an index or signature")
    raw, final = render(root, evidence, env)
    if mode == "verify":
        if read_regular(evidence / INDEX) != raw or read_regular(evidence / index.FINAL) != final:
            raise ValueError("standard release index is stale or altered")
    elif mode == "write":
        index_path = evidence / INDEX
        created_index = False
        if os.path.lexists(index_path):
            if read_regular(index_path) != raw:
                raise ValueError("existing partial index is stale or altered")
        else:
            index.publish_new(index_path, raw)
            created_index = True
        try:
            if candidate(root) != strict_json(raw)["candidate"]:
                raise ValueError("source changed before index publication")
            index.publish_new(evidence / index.FINAL, final)
        except BaseException:
            if created_index:
                index_path.unlink()
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    modes = parser.add_mutually_exclusive_group(required=True)
    for mode in ("template", "check", "write", "verify"):
        modes.add_argument("--" + mode, action="store_const", const=mode, dest="mode")
    args = parser.parse_args()
    root = scoped.campaign.ROOT
    try:
        run(root, root / metadata.PREFIX, os.environ, args.mode)
    except (ValueError, OSError, KeyError, TypeError, tarfile.TarError, subprocess.TimeoutExpired) as error:
        parser.exit(1, "standard release: " + str(error) + "\n")
    print("standard release: " + args.mode + " completed; no signing, tagging or publication performed")


if __name__ == "__main__":
    main()
