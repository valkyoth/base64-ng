#!/usr/bin/env python3
"""Validate the offline RFC 4648 source lock and requirements mapping."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import sys
from pathlib import Path


EXPECTED_RFC_SHA256 = "84e14418f795d503be5f34bf23ce4ebaa119e9ec7c9f667d8caeb111385b178f"
EXPECTED_ERRATA = {
    "2837": ("Verified", "Editorial", "16.2"),
    "5855": ("Verified", "Editorial", "10"),
    "7514": ("Verified", "Editorial", "3.5"),
    "4889": ("Reported", "Technical", "6"),
    "8669": ("Reported", "Technical", "4"),
    "9030": ("Verified", "Editorial", "5"),
}
REQUIRED_REQUIREMENTS = {
    "RFC4648-3.1-MUST-NO-UNSOLICITED-LINES",
    "RFC4648-3.2-MUST-PAD-BY-DEFAULT",
    "RFC4648-3.3-MUST-REJECT-NON-ALPHABET",
    "RFC4648-3.3-MAY-IGNORE-BY-REFERRING-SPEC",
    "RFC4648-3.3-MAY-IGNORE-EARLY-PADDING",
    "RFC4648-3.3-MAY-IGNORE-EXCESS-PADDING",
    "RFC4648-3.4-DEFAULT-NO-GLYPH-FOLDING",
    "RFC4648-3.5-MUST-ZERO-PAD-BITS",
    "RFC4648-3.5-MAY-REJECT-NONCANONICAL",
    "RFC4648-4-24-TO-4X6",
    "RFC4648-4-STANDARD-ALPHABET",
    "RFC4648-4-PADDING-CASES",
    "RFC4648-5-DISTINCT-NAME",
    "RFC4648-5-URL-SAFE-ALPHABET",
    "RFC4648-5-EXPLICIT-PADDING-OMISSION",
}
EXPECTED_VECTORS = [
    ("", ""),
    ("66", "Zg=="),
    ("666f", "Zm8="),
    ("666f6f", "Zm9v"),
    ("666f6f62", "Zm9vYg=="),
    ("666f6f6261", "Zm9vYmE="),
    ("666f6f626172", "Zm9vYmFy"),
]


def fail(message: str) -> None:
    raise ValueError(message)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_sources(directory: Path) -> None:
    rows = [
        line.split("\t")
        for line in (directory / "SOURCES").read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]
    expected = [
        "rfc4648.txt",
        "https://www.rfc-editor.org/rfc/rfc4648.txt",
        EXPECTED_RFC_SHA256,
    ]
    if expected not in rows or len(rows) != 5:
        fail("SOURCES does not contain the locked RFC 4648 HTTPS source")
    if digest(directory / "rfc4648.txt") != EXPECTED_RFC_SHA256:
        fail("rfc4648.txt differs from the locked RFC Editor bytes")
    rfc = (directory / "rfc4648.txt").read_bytes()
    if b"\r" in rfc:
        fail("rfc4648.txt line endings were normalized")
    if b"RFC 4648" not in rfc or b"Full Copyright Statement" not in rfc:
        fail("rfc4648.txt is missing identity or notice text")


def validate_errata(directory: Path) -> None:
    with (directory / "rfc4648-errata.tsv").open(
        newline="", encoding="utf-8"
    ) as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    required_columns = {
        "id",
        "status",
        "type",
        "section",
        "reported",
        "source",
        "reviewed",
        "disposition",
        "implementation_decision",
        "security_effect",
    }
    if not rows or set(rows[0]) != required_columns:
        fail("errata snapshot schema drifted")
    observed: dict[str, tuple[str, str, str]] = {}
    for row in rows:
        identifier = row["id"]
        if identifier in observed:
            fail(f"duplicate erratum {identifier}")
        observed[identifier] = (row["status"], row["type"], row["section"])
        if row["source"] != f"https://www.rfc-editor.org/errata/eid{identifier}":
            fail(f"erratum {identifier} source is not canonical HTTPS")
        for field in (
            "reported",
            "reviewed",
            "disposition",
            "implementation_decision",
            "security_effect",
        ):
            if not row[field]:
                fail(f"erratum {identifier} has empty {field}")
    if observed != EXPECTED_ERRATA:
        fail(f"errata status snapshot drifted: {observed!r}")


def validate_requirements(directory: Path) -> None:
    ledger = json.loads(
        (directory / "rfc4648-requirements.json").read_text(encoding="utf-8")
    )
    if ledger.get("schema_version") != 1 or ledger.get("rfc") != 4648:
        fail("requirements ledger identity drifted")
    alphabets = ledger.get("alphabets", {})
    if alphabets.get("standard") != (
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    ):
        fail("standard alphabet table drifted")
    if alphabets.get("url_safe") != (
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
    ):
        fail("URL-safe alphabet table drifted")
    policy = ledger.get("discrepancy_policy", {})
    if set(policy) != {
        "line_feeds",
        "padding",
        "non_alphabet",
        "trailing_bits",
        "alphabet",
    } or any(not value for value in policy.values()):
        fail("discrepancy policy is incomplete")

    requirements = ledger.get("requirements")
    if not isinstance(requirements, list):
        fail("requirements must be a list")
    observed: set[str] = set()
    for requirement in requirements:
        identifier = requirement.get("id")
        if not isinstance(identifier, str) or identifier in observed:
            fail(f"invalid or duplicate requirement id: {identifier!r}")
        observed.add(identifier)
        if not re.fullmatch(r"(3(\.[1-5])?|4|5)", requirement.get("section", "")):
            fail(f"{identifier} has an out-of-scope section")
        for field in ("normative", "decision"):
            if not requirement.get(field):
                fail(f"{identifier} has no {field}")
        for field in ("implementation", "tests"):
            references = requirement.get(field)
            if not isinstance(references, list) or not references:
                fail(f"{identifier} has no {field} mapping")
            for reference in references:
                if not Path(reference).is_file():
                    fail(f"{identifier} maps to missing file {reference}")
    if observed != REQUIRED_REQUIREMENTS:
        fail(
            "requirements ledger is missing or adds unlocked entries: "
            f"missing={sorted(REQUIRED_REQUIREMENTS - observed)}, "
            f"extra={sorted(observed - REQUIRED_REQUIREMENTS)}"
        )

    vectors = [
        (entry.get("input_hex"), entry.get("encoded"))
        for entry in ledger.get("official_base64_vectors", [])
    ]
    if vectors != EXPECTED_VECTORS:
        fail("official Base64 vector mapping drifted")


def validate_repository_policy() -> None:
    attributes = Path(".gitattributes").read_text(encoding="utf-8")
    if "/rfc/*.txt -text" not in attributes:
        fail(".gitattributes does not disable RFC text normalization")
    manifest = Path("Cargo.toml").read_text(encoding="utf-8")
    if '"rfc/**"' in manifest:
        fail("Cargo package include list contains rfc/**")
    lib = Path("src/v2/mod.rs").read_text(encoding="utf-8")
    if "#[cfg(test)]\nmod rfc4648_oracle;" not in lib:
        fail("independent oracle is not test-only")


def validate_npm_manifests() -> None:
    for manifest in Path(".").glob("**/package.json"):
        if "target" in manifest.parts or "node_modules" in manifest.parts:
            continue
        data = json.loads(manifest.read_text(encoding="utf-8"))
        files = data.get("files", [])
        if any("rfc" in str(entry).lower() for entry in files):
            fail(f"{manifest} npm files include RFC material")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--npm-packages", action="store_true")
    args = parser.parse_args()
    try:
        validate_sources(args.directory)
        validate_errata(args.directory)
        validate_requirements(args.directory)
        validate_repository_policy()
        if args.npm_packages:
            validate_npm_manifests()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"RFC 4648 policy: {error}", file=sys.stderr)
        return 1
    print("RFC 4648 policy: source, errata, and requirements ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
