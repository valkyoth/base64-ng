#!/usr/bin/env python3
"""Prove that representative RFC source-lock mutations fail offline checks."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Callable


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "rfc"
LOCKED_FILES = [
    "README.md",
    "SOURCES",
    "rfc2045-errata.tsv",
    "rfc2045-requirements.json",
    "rfc2045.txt",
    "rfc4648-errata.tsv",
    "rfc4648-requirements.json",
    "rfc4648.txt",
]


def relock(directory: Path) -> None:
    lines = []
    for name in LOCKED_FILES:
        digest = hashlib.sha256((directory / name).read_bytes()).hexdigest()
        lines.append(f"{digest}  {name}\n")
    (directory / "SHA256SUMS").write_text("".join(lines), encoding="ascii")


def expect_rejected(name: str, mutate: Callable[[Path], None]) -> None:
    with tempfile.TemporaryDirectory(prefix="base64-ng-rfc-mutation-") as temporary:
        candidate = Path(temporary) / "rfc"
        shutil.copytree(SOURCE, candidate)
        mutate(candidate)
        environment = os.environ.copy()
        environment["BASE64_NG_RFC_DIR"] = str(candidate)
        environment["BASE64_NG_RFC_SKIP_PACKAGE"] = "1"
        result = subprocess.run(
            [str(ROOT / "scripts/verify-rfcs.sh")],
            cwd=ROOT,
            env=environment,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            raise RuntimeError(f"RFC mutation was accepted: {name}")
        print(f"RFC mutation: rejected {name}")


def changed_bytes(directory: Path) -> None:
    path = directory / "rfc4648.txt"
    path.write_bytes(path.read_bytes() + b"changed")


def changed_line_endings(directory: Path) -> None:
    path = directory / "rfc4648.txt"
    path.write_bytes(path.read_bytes().replace(b"\n", b"\r\n"))
    relock(directory)


def changed_source(directory: Path) -> None:
    path = directory / "SOURCES"
    path.write_text(
        path.read_text(encoding="utf-8").replace(
            "https://www.rfc-editor.org/", "http://www.rfc-editor.org/"
        ),
        encoding="utf-8",
    )
    relock(directory)


def changed_checksum(directory: Path) -> None:
    path = directory / "SHA256SUMS"
    text = path.read_text(encoding="ascii")
    path.write_text(("0" * 64) + text[64:], encoding="ascii")


def missing_file(directory: Path) -> None:
    (directory / "rfc4648.txt").unlink()


def extra_file(directory: Path) -> None:
    (directory / "unlocked.txt").write_text("not locked\n", encoding="ascii")


def empty_file(directory: Path) -> None:
    (directory / "rfc4648.txt").write_bytes(b"")
    relock(directory)


def stale_errata(directory: Path) -> None:
    path = directory / "rfc4648-errata.tsv"
    path.write_text(
        path.read_text(encoding="utf-8").replace(
            "8669\tReported\t", "8669\tVerified\t"
        ),
        encoding="utf-8",
    )
    relock(directory)


def unmapped_requirement(directory: Path) -> None:
    path = directory / "rfc4648-requirements.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    data["requirements"][0]["tests"] = []
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    relock(directory)


def changed_rfc2045_bytes(directory: Path) -> None:
    path = directory / "rfc2045.txt"
    path.write_bytes(path.read_bytes() + b"changed")


def stale_rfc2045_errata(directory: Path) -> None:
    path = directory / "rfc2045-errata.tsv"
    path.write_text(
        path.read_text(encoding="utf-8").replace(
            "512\tVerified\t", "512\tReported\t"
        ),
        encoding="utf-8",
    )
    relock(directory)


def unmapped_rfc2045_requirement(directory: Path) -> None:
    path = directory / "rfc2045-requirements.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    data["requirements"][0]["implementation"] = []
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    relock(directory)


def main() -> int:
    mutations = [
        ("changed bytes", changed_bytes),
        ("changed line endings", changed_line_endings),
        ("non-HTTPS source URL", changed_source),
        ("changed checksum", changed_checksum),
        ("missing file", missing_file),
        ("extra file", extra_file),
        ("empty reference", empty_file),
        ("stale errata status", stale_errata),
        ("unmapped requirement", unmapped_requirement),
        ("changed RFC 2045 bytes", changed_rfc2045_bytes),
        ("stale RFC 2045 errata status", stale_rfc2045_errata),
        ("unmapped RFC 2045 requirement", unmapped_rfc2045_requirement),
    ]
    for name, mutation in mutations:
        expect_rejected(name, mutation)
    print(f"RFC mutation: {len(mutations)} fail-closed cases passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
