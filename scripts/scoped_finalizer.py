"""Prepare a claim-aware review index, never a final or sealable manifest."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile

import scoped_campaign as campaign
from scoped_evidence import read_regular, strict_json, verify_packet


POLICY_COMMIT = "1065a1f34973a21ab67e80141e42eb2dbf0edb65"
POLICY_TREE = "643c035113abf92691a0812c891e74fd9e8a9264"
HANDOFF_SHA256 = "986d4e087acd5da0b677458efcd31ce26cefbbf4097f4fe30348fb6467ac6c70"
REVIEW_RECORDS = {
    "policy-review.json", "recapture-review.json", "mechanical-assembly-review.json",
    "assembly-review.json", "finalization-readiness.json", "github-ci.json", "github-runs.json",
}
CLAIM_REVIEWS = {
    **{name: ["policy-review.json"] for name in (
        "codec-fuzz", "bounded-proofs", "ownership-miri", "memory-sanitizers",
        "big-endian-codec", "wasm-kernel", "reporting-native", "reporting-followups")},
    **{name: ["recapture-review.json"] for name in (
        "native-abi-admission", "timing", "paired-performance", "rvv-sve-emulation",
        "wasm-package-browsers")},
    "assembly": ["recapture-review.json", "mechanical-assembly-review.json", "assembly-review.json"],
}
# This permits preparation for review, not approval of these integration edits.
# No runtime, test harness, dependency, package or workflow change is allowed.
INTEGRATION_FILES = {
    "scripts/finalize-release-evidence.sh", "scripts/scoped_finalizer.py",
    "scripts/test-scoped-finalizer.py", "scripts/checks.sh",
    "docs/RELEASE_EVIDENCE.md", "docs/RELEASE_FREEZE_2.1.md",
    "scripts/scoped_index.py", "scripts/scoped_candidate.py",
    "scripts/test-scoped-index.py", "scripts/validate-release-readiness.sh",
    "scripts/run-scoped-python.sh", "scripts/test-scoped-python.py",
}
FROZEN_POLICY_FILES = campaign.POLICY_FILES - INTEGRATION_FILES


def record(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def clean_head(root):
    if campaign.git(root, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("integration worktree is dirty")
    # Git status can hide changed bytes behind these index flags.
    entries = campaign.git(root, "ls-files", "-v", "-z").split(b"\0")
    if any(entry and not entry.startswith(b"H ") for entry in entries):
        raise ValueError("integration index has hidden or unmerged entries")
    return campaign.git(root, "rev-parse", "HEAD").decode().strip()


def verify_candidate(root, policy, integration=None):
    candidate = clean_head(root)
    report = "security/pentest/v2.1.0.md"
    if integration is not None:
        campaign.commit_id(integration)
        if candidate != integration:
            parent = campaign.git(root, "show", "-s", "--format=%P", candidate).decode().strip()
            if parent != integration or campaign.changed_paths(root, candidate) != [report]:
                raise ValueError("candidate is neither reviewed integration nor its report-only child")
    if campaign.git(root, "rev-parse", POLICY_COMMIT + "^{tree}").decode().strip() != POLICY_TREE:
        raise ValueError("approved policy tree mismatch")
    for name in FROZEN_POLICY_FILES:
        if read_regular(root / name) != campaign.git(root, "show", POLICY_COMMIT + ":" + name):
            raise ValueError("approved policy implementation drift: " + name)
    campaign.verify_history(root, policy)
    parent = policy["history"][-1]["commit"]
    if campaign.git(root, "show", "-s", "--format=%P", POLICY_COMMIT).decode().strip() != parent:
        raise ValueError("approved policy parent mismatch")
    if set(campaign.changed_paths(root, POLICY_COMMIT)) != campaign.POLICY_CHANGE_FILES:
        raise ValueError("approved policy path inventory mismatch")
    campaign.git(root, "merge-base", "--is-ancestor", POLICY_COMMIT, candidate)
    revisions = campaign.git(root, "rev-list", "--reverse", POLICY_COMMIT + ".." + candidate)
    revisions = revisions.decode().splitlines()
    if not revisions:
        raise ValueError("missing committed finalizer integration")
    with tempfile.TemporaryDirectory(prefix="base64-finalizer-signers-") as directory:
        signers = Path(directory) / "allowed-signers"
        signers.write_bytes(campaign.git(root, "show", policy["history"][0]["commit"]
                                        + ":security/release-signers"))
        campaign.require_signature(root, POLICY_COMMIT, signers)
        previous = POLICY_COMMIT
        for revision in revisions:
            if campaign.git(root, "show", "-s", "--format=%P", revision).decode().strip() != previous:
                raise ValueError("integration history is not linear")
            paths = set(campaign.changed_paths(root, revision))
            report_only = integration is not None and revision == candidate and paths == {report}
            if not paths or (not paths <= INTEGRATION_FILES and not report_only):
                raise ValueError("integration history changed a protected path")
            if report_only and read_regular(root / report) != campaign.git(root, "show", candidate + ":" + report):
                raise ValueError("report-only child bytes differ from signed candidate")
            campaign.require_signature(root, revision, signers)
            previous = revision
    for name in INTEGRATION_FILES:
        if read_regular(root / name) != campaign.git(root, "show", candidate + ":" + name):
            raise ValueError("executing integration differs from candidate: " + name)
    return {"commit": candidate,
            "tree": campaign.git(root, "rev-parse", candidate + "^{tree}").decode().strip()}


def verify_reviews(directory, policy):
    raw = read_regular(directory / "review-handoff.json")
    if record(raw)["sha256"] != HANDOFF_SHA256:
        raise ValueError("independently approved handoff digest mismatch")
    handoff = strict_json(raw)
    if (handoff["policy_commit"] != POLICY_COMMIT or handoff["policy_tree"] != POLICY_TREE
            or handoff["release_authorized"] is not False
            or handoff["finalizer_integration_approved"] is not False):
        raise ValueError("handoff policy identity or authority mismatch")
    if set(handoff["selected_records"]) != REVIEW_RECORDS:
        raise ValueError("exact seven reviewed records are required")
    if (set(handoff["claim_dispositions"]) != set(CLAIM_REVIEWS)
            or {claim["id"] for claim in policy["claims"]} != set(CLAIM_REVIEWS)):
        raise ValueError("exact fourteen approved claims are required")
    for name, pin in handoff["selected_records"].items():
        raw = read_regular(directory / name)
        if record(raw) != pin:
            raise ValueError("reviewed record digest mismatch: " + name)
        strict_json(raw)
    return handoff


def prepare(root, reviews, paths, integration=None):
    policy = campaign.load_policy(root)
    candidate = verify_candidate(root, policy, integration)
    return prepare_claims(root, reviews, paths, policy, candidate)


def prepare_claims(root, reviews, paths, policy, candidate):
    """Verify retained claims without assigning new-source execution or authority."""
    handoff = verify_reviews(reviews, policy)
    if set(paths) != set(policy["packets"]):
        raise ValueError("exact historical and recapture packets are required")
    manifests = {name: verify_packet(paths[name], pin) for name, pin in policy["packets"].items()}
    summaries = campaign.load_attestations(root, policy)
    campaign.verify_claims(policy, manifests, summaries)
    claims = []
    for claim in policy["claims"]:
        claims.append({
            "claim": claim,
            "original_source": policy["sources"][claim["source"]],
            "attestation": summaries["attestations"][claim["id"]],
            "review_records": {name: handoff["selected_records"][name]
                               for name in CLAIM_REVIEWS[claim["id"]]},
            "approved_disposition": handoff["claim_dispositions"][claim["id"]],
        })
    if clean_head(root) != candidate["commit"]:
        raise ValueError("candidate changed during scoped preparation")
    return {
        "schema": 1,
        "evidence_mode": "scoped-review-only",
        "status": "prepared-for-separate-integration-review",
        "release_authorized": False,
        "finalization_permitted": False,
        "finalizer_integration_approved": False,
        "candidate_validated": False,
        "candidate": candidate,
        "approved_policy": {"commit": POLICY_COMMIT, "tree": POLICY_TREE},
        "scoped_claim_approval": {
            "basis": "maintainer-supplied separate-review approval of the exact handoff and records",
            "handoff_sha256": HANDOFF_SHA256,
            "selected_records": handoff["selected_records"],
            "boundary": "hash-bound scoped approval, not third-party certification or release authorization",
        },
        "packets": {name: {"archive": pin["archive"], "manifest": pin["manifest"]}
                    for name, pin in policy["packets"].items()},
        "claims": claims,
        "conditions_preserved": handoff["conditions_preserved"],
        "assembly_boundary": "the identified bounded assembly gaps are closed; all report exclusions remain",
        "pending": ["separate-integration-review", "final-candidate-metadata",
                    "exact-candidate-full-release-ci", "signed-final-index", "release-authorization"],
        "ci_boundary": "the approved 32-success/4-skipped record covers the policy, not final-candidate CI",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reviews", type=Path, required=True)
    parser.add_argument("--historical-packet", type=Path, required=True)
    parser.add_argument("--recapture-packet", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = prepare(campaign.ROOT, args.reviews,
                         {"historical": args.historical_packet, "recapture": args.recapture_packet})
    except (ValueError, OSError, KeyError, TypeError, tarfile.TarError, subprocess.TimeoutExpired) as error:
        parser.exit(1, "scoped finalizer review: " + str(error) + "\n")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
