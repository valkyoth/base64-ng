#!/usr/bin/env python3
"""Mutation tests for signed release-evidence artifact inventories."""

from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
from pathlib import Path


SOURCE = Path(__file__).resolve().with_name("verify-release-evidence-artifacts.py")


def run(repo: Path, succeeds: bool) -> None:
    result = subprocess.run(
        ["scripts/verify-release-evidence-artifacts.py"],
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if (result.returncode == 0) != succeeds:
        raise AssertionError(result.stdout + result.stderr)


def write_manifest(repo: Path) -> None:
    root = repo / "target/release-evidence"
    artifact = root / "campaign/result.txt"
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
    (root / "FINAL-MANIFEST.txt").write_text(
        "base64-ng final release evidence index\n\n"
        "artifact-hashes:\n"
        f"{digest}  target/release-evidence/campaign/result.txt\n",
        encoding="utf-8",
    )


def main() -> None:
    with tempfile.TemporaryDirectory() as raw:
        repo = Path(raw) / "repo"
        scripts = repo / "scripts"
        evidence = repo / "target/release-evidence"
        artifact = evidence / "campaign/result.txt"
        scripts.mkdir(parents=True)
        artifact.parent.mkdir(parents=True)
        shutil.copy2(SOURCE, scripts / SOURCE.name)
        artifact.write_text("verified evidence\n", encoding="utf-8")
        write_manifest(repo)
        run(repo, True)

        artifact.write_text("tampered\n", encoding="utf-8")
        run(repo, False)
        artifact.write_text("verified evidence\n", encoding="utf-8")

        extra = evidence / "unlisted.txt"
        extra.write_text("not signed\n", encoding="utf-8")
        run(repo, False)
        extra.unlink()

        external = repo / "external.txt"
        external.write_text("verified evidence\n", encoding="utf-8")
        artifact.unlink()
        artifact.symlink_to(external)
        run(repo, False)
        artifact.unlink()
        artifact.write_text("verified evidence\n", encoding="utf-8")

        original = evidence.with_name("release-evidence-real")
        evidence.rename(original)
        evidence.symlink_to(original, target_is_directory=True)
        run(repo, False)

    print("release evidence artifact tests: ok")


if __name__ == "__main__":
    main()
