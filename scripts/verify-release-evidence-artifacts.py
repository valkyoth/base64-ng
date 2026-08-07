#!/usr/bin/env python3
"""Verify every artifact covered by a final release-evidence manifest."""

from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[1]
HASH_LINE = re.compile(r"([0-9a-f]{64})  (target/release-evidence/[A-Za-z0-9._/-]+)")


def fail(message: str) -> None:
    raise SystemExit(f"release evidence artifacts: {message}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    if len(sys.argv) not in {1, 3}:
        fail("usage: verify-release-evidence-artifacts.py [manifest evidence-root]")
    manifest = Path(sys.argv[1]) if len(sys.argv) == 3 else Path(
        "target/release-evidence/FINAL-MANIFEST.txt"
    )
    evidence_root = Path(sys.argv[2]) if len(sys.argv) == 3 else Path(
        "target/release-evidence"
    )
    manifest = ROOT / manifest if not manifest.is_absolute() else manifest
    evidence_root = ROOT / evidence_root if not evidence_root.is_absolute() else evidence_root

    if evidence_root.is_symlink() or not evidence_root.is_dir():
        fail("evidence root is not a regular directory")
    if manifest.is_symlink() or not manifest.is_file():
        fail("manifest is not a regular file")

    lines = manifest.read_text(encoding="utf-8").splitlines()
    markers = [index for index, line in enumerate(lines) if line == "artifact-hashes:"]
    if len(markers) != 1:
        fail("manifest has no singleton artifact-hashes section")

    recorded: dict[str, str] = {}
    for line in lines[markers[0] + 1 :]:
        if not line:
            continue
        match = HASH_LINE.fullmatch(line)
        if match is None:
            fail(f"malformed artifact hash line: {line}")
        digest, relative = match.groups()
        path = PurePosixPath(relative)
        if path.is_absolute() or ".." in path.parts or relative in recorded:
            fail(f"invalid or duplicate artifact path: {relative}")
        recorded[relative] = digest
    if not recorded:
        fail("manifest contains no artifact hashes")

    excluded = {manifest, Path(f"{manifest}.sig")}
    actual: dict[str, Path] = {}
    for entry in evidence_root.rglob("*"):
        if entry.is_symlink():
            fail(f"evidence tree contains a symbolic link: {entry}")
        if entry.is_dir():
            continue
        if not entry.is_file():
            fail(f"evidence tree contains a non-regular entry: {entry}")
        if entry in excluded:
            continue
        relative = entry.relative_to(ROOT).as_posix()
        actual[relative] = entry

    if set(actual) != set(recorded):
        fail("signed artifact inventory does not match the evidence tree")
    for relative, path in actual.items():
        if sha256_file(path) != recorded[relative]:
            fail(f"artifact checksum mismatch: {relative}")

    print("release evidence artifacts: signed inventory and checksums verified")


if __name__ == "__main__":
    main()
