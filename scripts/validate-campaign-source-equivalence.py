#!/usr/bin/env python3
"""Verify that retained external campaigns still cover the current candidate."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FULL_COMMIT = re.compile(r"[0-9a-f]{40}")
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
    if git("rev-list", "--count", f"{campaign}..{candidate}") != "1":
        fail("candidate must be the single immediate correction commit")

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
    if changed != ALLOWED_TOOLING_CHANGES:
        missing = sorted(ALLOWED_TOOLING_CHANGES - changed)
        unexpected = sorted(changed - ALLOWED_TOOLING_CHANGES)
        fail(f"correction inventory mismatch: missing={missing} unexpected={unexpected}")

    print(
        "campaign source equivalence: runtime, crates, tests, fuzz inputs, "
        "dependencies, and toolchain are unchanged"
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
