#!/usr/bin/env python3
"""Validate the offline RFC 3501 modified-Base64 evidence lock."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path


EXPECTED_RFC_SHA256 = "4437974dde85859f2391f32215295e7b20ce157c3bd2adbe7e5b443e990f15cf"
EXPECTED_ERRATA = {"261": ("Verified", "Technical", "5.1.3")}
REQUIREMENTS = {
    "RFC3501-5.1.3-ALPHABET",
    "RFC3501-5.1.3-UTF16BE",
    "RFC3501-5.1.3-EXACT-TEXT",
    "RFC3501-5.1.3-SCOPE",
}


def fail(message: str) -> None:
    raise ValueError(message)


def validate(directory: Path) -> None:
    source = directory / "rfc3501.txt"
    if hashlib.sha256(source.read_bytes()).hexdigest() != EXPECTED_RFC_SHA256:
        fail("rfc3501.txt differs from the locked RFC Editor bytes")
    text = source.read_bytes()
    for marker in (
        b"RFC 3501",
        b"5.1.3.  Mailbox International Naming Convention",
        b"represented in modified BASE64",
        b'"," is used instead of "/"',
        b"MUST preserve the exact form of the modified BASE64 portion",
    ):
        if marker not in text:
            fail(f"RFC source is missing marker {marker!r}")

    source_rows = [
        line.split("\t")
        for line in (directory / "SOURCES").read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]
    expected_source = [
        "rfc3501.txt",
        "https://www.rfc-editor.org/rfc/rfc3501.txt",
        EXPECTED_RFC_SHA256,
    ]
    if expected_source not in source_rows or len(source_rows) != 5:
        fail("SOURCES does not contain the locked RFC 3501 HTTPS source")

    with (directory / "rfc3501-errata.tsv").open(
        newline="", encoding="utf-8"
    ) as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    observed = {
        row["id"]: (row["status"], row["type"], row["section"]) for row in rows
    }
    if observed != EXPECTED_ERRATA:
        fail(f"RFC 3501 relevant errata snapshot drifted: {observed!r}")
    for row in rows:
        if row["source"] != f"https://www.rfc-editor.org/errata/eid{row['id']}":
            fail(f"erratum {row['id']} source is not canonical HTTPS")
        if any(not row[field] for field in row):
            fail(f"erratum {row['id']} has incomplete disposition metadata")

    ledger = json.loads(
        (directory / "rfc3501-requirements.json").read_text(encoding="utf-8")
    )
    if ledger.get("schema_version") != 1 or ledger.get("rfc") != 3501:
        fail("RFC 3501 requirements identity drifted")
    requirements = ledger.get("requirements", [])
    identifiers = {entry.get("id") for entry in requirements}
    if identifiers != REQUIREMENTS or len(requirements) != len(REQUIREMENTS):
        fail("RFC 3501 requirement set drifted")
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
        print(f"RFC 3501 policy: {error}", file=sys.stderr)
        return 1
    print("RFC 3501 policy: legacy modified-Base64 source, errata, and requirements ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
