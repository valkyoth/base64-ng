#!/usr/bin/env python3
"""Validate the pinned Base64-family multibase source corpus offline."""

from __future__ import annotations

import csv
import hashlib
import pathlib
import sys


PINNED_COMMIT = "d7406cdea189b82a0b3937f5737b440f5fa92f92"
EXPECTED_REGISTRY = {
    "base64": ("U+006d", "m", "RFC4648 no padding", "final"),
    "base64pad": (
        "U+004d",
        "M",
        "RFC4648 with padding - MIME encoding",
        "experimental",
    ),
    "base64url": ("U+0075", "u", "RFC4648 no padding", "final"),
    "base64urlpad": ("U+0055", "U", "RFC4648 with padding", "final"),
}
EXPECTED_VECTORS = {
    "basic.csv": {
        "base64": "meWVzIG1hbmkgIQ",
        "base64pad": "MeWVzIG1hbmkgIQ==",
        "base64url": "ueWVzIG1hbmkgIQ",
        "base64urlpad": "UeWVzIG1hbmkgIQ==",
    },
    "leading_zero.csv": {
        "base64": "mAHllcyBtYW5pICE",
        "base64pad": "MAHllcyBtYW5pICE=",
        "base64url": "uAHllcyBtYW5pICE",
        "base64urlpad": "UAHllcyBtYW5pICE=",
    },
    "two_leading_zeros.csv": {
        "base64": "mAAB5ZXMgbWFuaSAh",
        "base64pad": "MAAB5ZXMgbWFuaSAh",
        "base64url": "uAAB5ZXMgbWFuaSAh",
        "base64urlpad": "UAAB5ZXMgbWFuaSAh",
    },
}


def fail(message: str) -> None:
    raise SystemExit(f"multibase spec: {message}")


def parse_checksums(root: pathlib.Path) -> dict[str, str]:
    checksums: dict[str, str] = {}
    for line in (root / "SHA256SUMS").read_text(encoding="ascii").splitlines():
        digest, name = line.split("  ", 1)
        checksums[name] = digest
    return checksums


def validate_hashes(root: pathlib.Path) -> None:
    expected = parse_checksums(root)
    required = {
        "COMMIT",
        "SOURCES",
        "upstream-README.md",
        "multibase.csv",
        "tests/basic.csv",
        "tests/leading_zero.csv",
        "tests/two_leading_zeros.csv",
    }
    if set(expected) != required:
        fail("checksum inventory does not exactly match the pinned corpus")
    for name, digest in expected.items():
        path = root / name
        if not path.is_file():
            fail(f"missing pinned file: {name}")
        observed = hashlib.sha256(path.read_bytes()).hexdigest()
        if observed != digest:
            fail(f"checksum mismatch: {name}")


def validate_sources(root: pathlib.Path) -> None:
    if (root / "COMMIT").read_text(encoding="ascii").strip() != PINNED_COMMIT:
        fail("pinned commit changed")
    sources = (root / "SOURCES").read_text(encoding="ascii").splitlines()
    if len(sources) != 6 or any(PINNED_COMMIT not in source for source in sources):
        fail("source inventory is not fully commit-pinned")
    if any(not source.startswith("https://") for source in sources):
        fail("source inventory contains a non-HTTPS URL")


def validate_registry(root: pathlib.Path) -> None:
    rows: dict[str, tuple[str, str, str, str]] = {}
    with (root / "multibase.csv").open(encoding="utf-8", newline="") as source:
        for row in csv.DictReader(source, skipinitialspace=True):
            encoding = row["encoding"].strip()
            if encoding.startswith("base64"):
                rows[encoding] = (
                    row["Unicode"].strip(),
                    row["character"].strip(),
                    row["description"].strip(),
                    row["status"].strip(),
                )
    if rows != EXPECTED_REGISTRY:
        fail("Base64-family registry rows changed")


def validate_vectors(root: pathlib.Path) -> None:
    for filename, expected in EXPECTED_VECTORS.items():
        with (root / "tests" / filename).open(encoding="utf-8", newline="") as source:
            rows = {
                row["encoding"].strip(): row[next(key for key in row if key != "encoding")]
                for row in csv.DictReader(source, skipinitialspace=True)
            }
        observed = {name: rows.get(name) for name in expected}
        if observed != expected:
            fail(f"official Base64-family vectors changed: {filename}")


def main() -> None:
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "spec/multibase")
    validate_hashes(root)
    validate_sources(root)
    validate_registry(root)
    validate_vectors(root)
    print("multibase spec: pinned commit, registry, and Base64-family vectors ok")


if __name__ == "__main__":
    main()
