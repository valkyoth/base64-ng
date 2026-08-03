#!/usr/bin/env python3
"""Prove representative multibase source-lock mutations fail closed."""

from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Callable


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "spec" / "multibase"


def relock(directory: Path) -> None:
    entries = []
    for line in (directory / "SHA256SUMS").read_text(encoding="ascii").splitlines():
        _, name = line.split("  ", 1)
        digest = hashlib.sha256((directory / name).read_bytes()).hexdigest()
        entries.append(f"{digest}  {name}\n")
    (directory / "SHA256SUMS").write_text("".join(entries), encoding="ascii")


def expect_rejected(name: str, mutate: Callable[[Path], None]) -> None:
    with tempfile.TemporaryDirectory(prefix="base64-ng-multibase-mutation-") as temporary:
        candidate = Path(temporary) / "multibase"
        shutil.copytree(SOURCE, candidate)
        mutate(candidate)
        result = subprocess.run(
            [str(ROOT / "scripts/validate-multibase-spec.py"), str(candidate)],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            raise RuntimeError(f"multibase mutation was accepted: {name}")
        print(f"multibase mutation: rejected {name}")


def changed_commit(directory: Path) -> None:
    (directory / "COMMIT").write_text("0" * 40 + "\n", encoding="ascii")
    relock(directory)


def unpinned_source(directory: Path) -> None:
    path = directory / "SOURCES"
    path.write_text(
        path.read_text(encoding="ascii").replace(
            "d7406cdea189b82a0b3937f5737b440f5fa92f92", "master"
        ),
        encoding="ascii",
    )
    relock(directory)


def changed_registry_bytes(directory: Path) -> None:
    path = directory / "multibase.csv"
    path.write_bytes(path.read_bytes() + b"changed\n")


def changed_registry_semantics(directory: Path) -> None:
    path = directory / "multibase.csv"
    path.write_text(
        path.read_text(encoding="utf-8").replace(
            "U+006d,     m,          base64,", "U+006d,     x,          base64,"
        ),
        encoding="utf-8",
    )
    relock(directory)


def changed_vector_semantics(directory: Path) -> None:
    path = directory / "tests" / "basic.csv"
    path.write_text(
        path.read_text(encoding="utf-8").replace(
            'base64, "meWVzIG1hbmkgIQ"', 'base64, "MeWVzIG1hbmkgIQ"'
        ),
        encoding="utf-8",
    )
    relock(directory)


def missing_vector(directory: Path) -> None:
    (directory / "tests" / "leading_zero.csv").unlink()


def main() -> None:
    cases = [
        ("changed commit", changed_commit),
        ("unpinned source", unpinned_source),
        ("changed registry bytes", changed_registry_bytes),
        ("changed registry semantics", changed_registry_semantics),
        ("changed vector semantics", changed_vector_semantics),
        ("missing vector", missing_vector),
    ]
    for name, mutate in cases:
        expect_rejected(name, mutate)
    print(f"multibase mutation: {len(cases)} fail-closed cases passed")


if __name__ == "__main__":
    main()
