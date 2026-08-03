#!/usr/bin/env python3
"""Validate the offline RFC 2045 Section 6.8 evidence lock."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path


EXPECTED_RFC_SHA256 = "9bb251635dd37fda97dcce6c08dea019432117a0f1e389051d2ecbf7b76350b0"
EXPECTED_ERRATA = {
    "512": ("Verified", "Editorial", "5.1"),
    "2586": ("Verified", "Technical", "1"),
    "7120": ("Verified", "Editorial", "2.4"),
}
REQUIREMENTS = {
    "RFC2045-6.8-STANDARD-TABLE-1",
    "RFC2045-6.8-MAX-76-COLUMNS",
    "RFC2045-6.8-IGNORE-NONALPHABET",
    "RFC2045-6.8-CANONICAL-PADDING",
    "RFC2045-6.8-TEXT-CANONICALIZATION-OUTSIDE",
}


def fail(message: str) -> None:
    raise ValueError(message)


def validate(directory: Path) -> None:
    source = directory / "rfc2045.txt"
    if hashlib.sha256(source.read_bytes()).hexdigest() != EXPECTED_RFC_SHA256:
        fail("rfc2045.txt differs from the locked RFC Editor bytes")
    text = source.read_bytes()
    for marker in (
        b"RFC 2045",
        b"6.8.  Base64 Content-Transfer-Encoding",
        b"Any characters outside of the base64 alphabet are to be ignored",
    ):
        if marker not in text:
            fail(f"RFC source is missing marker {marker!r}")
    source_rows = [
        line.split("\t")
        for line in (directory / "SOURCES").read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]
    expected_source = [
        "rfc2045.txt",
        "https://www.rfc-editor.org/rfc/rfc2045.txt",
        EXPECTED_RFC_SHA256,
    ]
    if expected_source not in source_rows or len(source_rows) != 2:
        fail("SOURCES does not contain the locked RFC 2045 HTTPS source")

    with (directory / "rfc2045-errata.tsv").open(
        newline="", encoding="utf-8"
    ) as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    observed = {
        row["id"]: (row["status"], row["type"], row["section"]) for row in rows
    }
    if observed != EXPECTED_ERRATA:
        fail(f"RFC 2045 errata snapshot drifted: {observed!r}")
    for row in rows:
        if row["source"] != f"https://www.rfc-editor.org/errata/eid{row['id']}":
            fail(f"erratum {row['id']} source is not canonical HTTPS")
        if any(not row[field] for field in row):
            fail(f"erratum {row['id']} has incomplete disposition metadata")

    ledger = json.loads(
        (directory / "rfc2045-requirements.json").read_text(encoding="utf-8")
    )
    if ledger.get("schema_version") != 1 or ledger.get("rfc") != 2045:
        fail("RFC 2045 requirements identity drifted")
    requirements = ledger.get("requirements", [])
    identifiers = {entry.get("id") for entry in requirements}
    if identifiers != REQUIREMENTS or len(requirements) != len(REQUIREMENTS):
        fail("RFC 2045 requirement set drifted")
    for entry in requirements:
        if entry.get("section") != "6.8":
            fail(f"{entry.get('id')} escaped Section 6.8 scope")
        for field in ("normative", "decision"):
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
        print(f"RFC 2045 policy: {error}", file=sys.stderr)
        return 1
    print("RFC 2045 policy: Section 6.8 source, errata, and requirements ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
