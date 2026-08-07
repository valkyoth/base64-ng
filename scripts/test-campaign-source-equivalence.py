#!/usr/bin/env python3
"""Mutation tests for the narrowly scoped external-campaign reuse policy."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts" / "validate-campaign-source-equivalence.py"
SPEC = importlib.util.spec_from_file_location("campaign_source_validator", VALIDATOR)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR_MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = VALIDATOR_MODULE
SPEC.loader.exec_module(VALIDATOR_MODULE)


def git(repo: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def commit(repo: Path, path: str, content: str, message: str) -> str:
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    git(repo, "add", path)
    git(repo, "commit", "-m", message)
    return git(repo, "rev-parse", "HEAD")


def run(repo: Path, campaign: str, success: bool) -> None:
    original = VALIDATOR_MODULE.ROOT
    VALIDATOR_MODULE.ROOT = repo
    try:
        try:
            VALIDATOR_MODULE.validate(campaign, "HEAD")
        except SystemExit:
            if success:
                raise
        else:
            if not success:
                raise SystemExit("campaign source equivalence mutation passed")
    finally:
        VALIDATOR_MODULE.ROOT = original


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        repo = Path(directory) / "repo"
        repo.mkdir()
        git(repo, "init", "-q")
        git(repo, "config", "user.name", "fixture")
        git(repo, "config", "user.email", "fixture@example.invalid")
        campaign = commit(repo, "src/lib.rs", "pub fn value() {}\n", "campaign")
        report = "security/pentest/v2.0.0.md"
        for path in sorted(VALIDATOR_MODULE.ALLOWED_TOOLING_CHANGES - {report}):
            target = repo / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(f"fixture {path}\n", encoding="utf-8")
        git(repo, "add", ".")
        git(repo, "commit", "-m", "reviewed tooling correction")

        commit(repo, report, "final reviewed report\n", "report-only release commit")
        run(repo, campaign, True)

        (repo / "src/lib.rs").write_text("pub fn changed() {}\n", encoding="utf-8")
        git(repo, "add", "src/lib.rs")
        git(repo, "commit", "-m", "runtime mutation")
        run(repo, campaign, False)

    print("campaign source equivalence: mutation checks ok")


if __name__ == "__main__":
    main()
