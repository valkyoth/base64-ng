#!/usr/bin/env python3
"""Verify that retained external campaigns still cover the current candidate."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FULL_COMMIT = re.compile(r"[0-9a-f]{40}")
FROZEN_21_COMMIT = "b3e493e64a583245f7aab542d983b7b914c67eb9"
FROZEN_21_TREE = "06f9e65432631d43f868a95bb6083c18a61a1f39"
NEON_21_CORRECTION = {
    "scripts/check-2.0-memory-hardware-evidence.sh",
    "scripts/checks.sh",
    "scripts/evidence-equivalence.py",
    "scripts/finalize-release-evidence.sh",
    "scripts/test-campaign-source-equivalence.py",
    "scripts/test-neon-campaign.py",
    "scripts/test-release-evidence-outcomes.sh",
    "scripts/validate-campaign-source-equivalence.py",
    "scripts/validate-neon-campaign.py",
    "scripts/validate-release-metadata.sh",
    "scripts/validate-release-evidence-outcomes.sh",
}
NEON_21_METADATA = {"docs/RELEASE_FREEZE_2.1.md", "docs/RELEASE_EVIDENCE.md"}
ALLOWED_TOOLING_CHANGES = {
    "docs/RELEASE.md",
    "docs/RELEASE_EVIDENCE.md",
    "scripts/aggregate-fuzz-shards.sh",
    "scripts/checks.sh",
    "scripts/finalize-release-evidence.sh",
    "scripts/fuzz_shard_evidence.py",
    "scripts/stable_release_gate.sh",
    "scripts/test-campaign-source-equivalence.py",
    "scripts/test-fuzz-shard-evidence.py",
    "scripts/test-neon-admission-bundle.py",
    "scripts/validate-campaign-source-equivalence.py",
    "scripts/validate-neon-admission-bundle.py",
    "scripts/validate-release-metadata.sh",
    "security/pentest/v2.0.0.md",
}


def fail(message: str) -> None:
    raise SystemExit(f"campaign source equivalence: {message}")


def git(*arguments: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        fail(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout.strip()


def resolve(revision: str, label: str) -> str:
    commit = git("rev-parse", "--verify", f"{revision}^{{commit}}")
    if FULL_COMMIT.fullmatch(commit) is None:
        fail(f"{label} is not an exact commit")
    return commit


def validate(campaign_revision: str, candidate_revision: str) -> tuple[str, str]:
    campaign = resolve(campaign_revision, "campaign source")
    candidate = resolve(candidate_revision, "candidate source")
    if campaign == candidate:
        fail("campaign and candidate commits must differ")
    if subprocess.run(
        ["git", "merge-base", "--is-ancestor", campaign, candidate],
        cwd=ROOT,
        check=False,
    ).returncode != 0:
        fail("campaign source is not an ancestor of the candidate")
    if git("rev-list", "--merges", f"{campaign}..{candidate}"):
        fail("campaign-to-candidate range contains a merge commit")
    if candidate == resolve("HEAD", "HEAD") and git(
        "status", "--porcelain", "--untracked-files=all"
    ):
        fail("candidate worktree is not clean")

    changed = set(
        git(
            "diff",
            "--name-only",
            "--diff-filter=ACDMRTUXB",
            campaign,
            candidate,
        ).splitlines()
    )
    if not changed:
        fail("campaign-to-candidate range contains no changes")
    if campaign == FROZEN_21_COMMIT:
        if git("rev-parse", f"{campaign}^{{tree}}") != FROZEN_21_TREE:
            fail("2.1 frozen tree does not match the reviewed campaign")
        if changed - NEON_21_METADATA != NEON_21_CORRECTION:
            fail("2.1 native-inventory correction inventory mismatch")
        # Do not permit an intervening runtime change hidden by a later revert.
        corrections = 0
        for revision in git("rev-list", f"{campaign}..{candidate}").splitlines():
            touched = set(git("diff-tree", "--no-commit-id", "--name-only", "-r", revision).splitlines())
            if touched - NEON_21_CORRECTION - NEON_21_METADATA:
                fail("2.1 correction history touches protected source")
            tools = touched & NEON_21_CORRECTION
            if tools:
                if tools != NEON_21_CORRECTION:
                    fail("2.1 tooling correction must be one complete reviewed commit")
                corrections += 1
        if corrections != 1:
            fail("2.1 requires exactly one native-inventory tooling correction")
    elif changed != ALLOWED_TOOLING_CHANGES:
        missing = sorted(ALLOWED_TOOLING_CHANGES - changed)
        unexpected = sorted(changed - ALLOWED_TOOLING_CHANGES)
        fail(f"correction inventory mismatch: missing={missing} unexpected={unexpected}")

    print(
        "campaign source equivalence: runtime, crates, tests, fuzz inputs, "
        "dependencies, and toolchain are unchanged across the linear "
        "correction range"
    )
    return campaign, candidate


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--campaign", required=True)
    parser.add_argument("--candidate", default="HEAD")
    arguments = parser.parse_args()
    validate(arguments.campaign, arguments.candidate)


if __name__ == "__main__":
    main()
