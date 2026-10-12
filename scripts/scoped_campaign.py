"""Claim-specific review preflight. This module cannot authorize a release."""

import os
from pathlib import Path
import re
import subprocess
import tempfile

from scoped_evidence import canonical, read_regular, strict_json, verify_packet


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = "security/campaign-claims-2.1.json"
POLICY_ENV = "BASE64_NG_REVIEWED_SCOPED_POLICY_COMMIT"
POLICY_FILES = {
    POLICY_PATH,
    "scripts/scoped_campaign.py",
    "scripts/scoped_evidence.py",
    "scripts/validate-scoped-campaign.py",
    "scripts/test-scoped-campaign.py",
    "scripts/test-scoped-evidence.py",
    "scripts/checks.sh",
    "docs/RELEASE_FREEZE_2.1.md",
    "docs/RELEASE_EVIDENCE.md",
}
FULL_COMMIT = re.compile(r"[0-9a-f]{40}")


def git(root, *arguments):
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_NO_REPLACE_OBJECTS="1", GIT_NO_LAZY_FETCH="1", GIT_TERMINAL_PROMPT="0")
    result = subprocess.run(["git", "--no-replace-objects", *arguments], cwd=root,
                            env=env, capture_output=True, timeout=120, check=False)
    if result.returncode:
        raise ValueError("git check failed: " + result.stderr.decode(errors="replace").strip())
    return result.stdout


def commit_id(value):
    if not isinstance(value, str) or not FULL_COMMIT.fullmatch(value):
        raise ValueError("expected full commit/tree identity")
    return value


def changed_paths(root, revision):
    return git(root, "diff-tree", "--no-commit-id", "--no-renames", "--name-only",
               "-r", "-z", revision).decode().rstrip("\0").split("\0")


def require_signature(root, revision, signers):
    git(root, "-c", "gpg.format=ssh", "-c", "gpg.ssh.program=ssh-keygen",
        "-c", f"gpg.ssh.allowedSignersFile={signers}", "verify-commit", revision)


def verify_history(root, policy, anchor=None):
    history = policy["history"]
    if not isinstance(history, list) or len(history) < 2:
        raise ValueError("missing pinned history")
    ids = [commit_id(item["commit"]) for item in history]
    if len(set(ids)) != len(ids):
        raise ValueError("duplicate history commit")
    if git(root, "rev-list", "--reverse", ids[0] + ".." + ids[-1]).decode().splitlines() != ids[1:]:
        raise ValueError("pinned history identity/order mismatch")
    with tempfile.TemporaryDirectory(prefix="base64-scoped-signers-") as directory:
        signers = Path(directory) / "allowed-signers"
        signers.write_bytes(git(root, "show", ids[0] + ":security/release-signers"))
        for index, item in enumerate(history):
            revision = item["commit"]
            if git(root, "rev-parse", revision + "^{tree}").decode().strip() != commit_id(item["tree"]):
                raise ValueError("pinned history tree mismatch")
            require_signature(root, revision, signers)
            if index:
                if git(root, "show", "-s", "--format=%P", revision).decode().strip() != ids[index - 1]:
                    raise ValueError("history is not the exact linear chain")
                if changed_paths(root, revision) != item["paths"]:
                    raise ValueError("pinned correction path inventory mismatch")
        identities = {item["commit"]: item["tree"] for item in history}
        for source in policy["sources"].values():
            if identities.get(source["commit"]) != source["tree"]:
                raise ValueError("claim source is outside pinned history")
        if anchor is not None:
            commit_id(anchor)
            if git(root, "rev-parse", "HEAD").decode().strip() != anchor:
                raise ValueError("reviewed policy must be the exact current HEAD")
            if git(root, "status", "--porcelain", "--untracked-files=all"):
                raise ValueError("reviewed policy worktree is dirty")
            if git(root, "show", "-s", "--format=%P", anchor).decode().strip() != ids[-1]:
                raise ValueError("reviewed policy must directly follow pinned assessment")
            if set(changed_paths(root, anchor)) != POLICY_FILES:
                raise ValueError("reviewed policy path inventory mismatch")
            require_signature(root, anchor, signers)
            for name in POLICY_FILES:
                if read_regular(root / name) != git(root, "show", anchor + ":" + name):
                    raise ValueError("executing policy file differs from reviewed commit: " + name)


def verify_claims(policy, packets):
    if policy["schema"] != 1 or policy["release_authorized"] is not False:
        raise ValueError("not a non-authorizing scoped policy")
    claims = policy["claims"]
    if not isinstance(claims, list) or not claims:
        raise ValueError("missing claim inventory")
    ids = set()
    for claim in claims:
        if claim["id"] in ids:
            raise ValueError("duplicate claim identity")
        ids.add(claim["id"])
        if claim["source"] not in policy["sources"]:
            raise ValueError("unknown claim source")
        if claim["disposition"] not in (
                "historical-scoped-reuse", "fresh-reporting-supplement", "fresh-recapture"):
            raise ValueError("unknown claim disposition")
        if not claim["scope"] or not claim["excludes"] or not claim["evidence"]:
            raise ValueError("claim requires scope, exclusions and evidence")
        for reference in claim["evidence"]:
            manifest = packets[reference["packet"]]
            name = reference["path"]
            canonical(name)
            if name not in manifest["files"] and name not in manifest["directories"]:
                raise ValueError("claim references missing evidence: " + name)
            if name in manifest["directories"] and not any(
                    path.startswith(name + "/") for path in manifest["files"]):
                raise ValueError("claim references empty evidence directory")
    if set(policy["pending"]) != {
            "independent-policy-review", "independent-recapture-review", "final-candidate-metadata",
            "exact-candidate-full-release-ci", "signed-final-index", "release-authorization"}:
        raise ValueError("remaining release requirements changed")


def verify(root, policy, paths, anchor=None):
    if set(paths) != set(policy["packets"]):
        raise ValueError("exact historical and recapture packets are required")
    verify_history(root, policy, anchor)
    manifests = {name: verify_packet(paths[name], pin) for name, pin in policy["packets"].items()}
    verify_claims(policy, manifests)
    return {
        "schema": 1,
        "status": "reviewed-policy-preflight" if anchor else "proposal-preflight-not-approved",
        "release_authorized": False,
        "candidate_validated": anchor is not None,
        "policy_commit": anchor,
        "assessment_commit": policy["history"][-1]["commit"],
        "packets": {name: {"archive": pin["archive"], "manifest": pin["manifest"]}
                    for name, pin in policy["packets"].items()},
        "sources": policy["sources"],
        "claims": policy["claims"],
        "pending": policy["pending"],
    }


def load_policy(root):
    return strict_json(read_regular(root / POLICY_PATH))
