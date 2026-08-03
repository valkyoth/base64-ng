#!/usr/bin/env python3
"""Validate the offline RFC 9580 OpenPGP armor evidence lock."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path


EXPECTED_RFC_SHA256 = "36540e57b63d24c0b17be0748b704bdeed3d643ce99a4fb1abb6469c1eb1ff18"
EXPECTED_ERRATA = {
    "8432": ("Verified", "Editorial", "16.1"),
    "8449": ("Held for Document Update", "Technical", "5.11"),
    "8454": ("Verified", "Technical", "5.2.3"),
    "8465": ("Verified", "Technical", "5.5.3"),
    "8466": ("Verified", "Technical", "11.2"),
    "8814": ("Reported", "Technical", "5.6"),
    "8847": ("Reported", "Technical", "9.1"),
}
REQUIREMENTS = {
    "RFC9580-6-BASE64",
    "RFC9580-6.1-CRC24",
    "RFC9580-6.2-CONTAINER",
    "RFC9580-6.2.1-TYPES",
    "RFC9580-6.2.2-HEADERS",
    "RFC9580-6.2.2.1-METADATA",
    "RFC9580-7-CLEARTEXT-EXCLUSION",
}


def fail(message: str) -> None:
    raise ValueError(message)


def validate(directory: Path) -> None:
    source = directory / "rfc9580.txt"
    if hashlib.sha256(source.read_bytes()).hexdigest() != EXPECTED_RFC_SHA256:
        fail("rfc9580.txt differs from the locked RFC Editor bytes")
    text = source.read_bytes()
    for marker in (
        b"Request for Comments: 9580",
        b"OpenPGP",
        b"6.1.1.  An Implementation of the CRC24 in \"C\"",
        b"An Armor Header Line consists",
        b"MUST NOT reject an OpenPGP object when the CRC24",
    ):
        if marker not in text:
            fail(f"RFC source is missing marker {marker!r}")

    source_rows = [
        line.split("\t")
        for line in (directory / "SOURCES").read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]
    expected_source = [
        "rfc9580.txt",
        "https://www.rfc-editor.org/rfc/rfc9580.txt",
        EXPECTED_RFC_SHA256,
    ]
    if expected_source not in source_rows or len(source_rows) != 5:
        fail("SOURCES does not contain the locked RFC 9580 HTTPS source")

    with (directory / "rfc9580-errata.tsv").open(
        newline="", encoding="utf-8"
    ) as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    observed = {
        row["id"]: (row["status"], row["type"], row["section"]) for row in rows
    }
    if observed != EXPECTED_ERRATA:
        fail(f"RFC 9580 errata snapshot drifted: {observed!r}")
    for row in rows:
        if row["source"] != f"https://www.rfc-editor.org/errata/eid{row['id']}":
            fail(f"erratum {row['id']} source is not canonical HTTPS")
        if any(not row[field] for field in row):
            fail(f"erratum {row['id']} has incomplete disposition metadata")

    ledger = json.loads(
        (directory / "rfc9580-requirements.json").read_text(encoding="utf-8")
    )
    if ledger.get("schema_version") != 1 or ledger.get("rfc") != 9580:
        fail("RFC 9580 requirements identity drifted")
    requirements = ledger.get("requirements", [])
    identifiers = {entry.get("id") for entry in requirements}
    if identifiers != REQUIREMENTS or len(requirements) != len(REQUIREMENTS):
        fail("RFC 9580 requirement set drifted")
    for entry in requirements:
        for field in ("section", "normative", "decision"):
            if not entry.get(field):
                fail(f"{entry.get('id')} has no {field}")
        for field in ("implementation", "tests"):
            references = entry.get(field)
            if not references or any(not Path(path).is_file() for path in references):
                fail(f"{entry.get('id')} has invalid {field} references")


def main() -> int:
    try:
        validate(Path(sys.argv[1] if len(sys.argv) > 1 else "rfc"))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"RFC 9580 policy: {error}", file=sys.stderr)
        return 1
    print("RFC 9580 policy: OpenPGP armor source, errata, and requirements ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
