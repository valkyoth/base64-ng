#!/usr/bin/env python3
"""Validate the offline RFC 7468 textual-encoding evidence lock."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path


EXPECTED_RFC_SHA256 = "0b2c3c2087cc0b099789c90e61c0208e87b25793f0ce40090979e8c734b3d989"
EXPECTED_ERRATA = {
    "4508": ("Verified", "Technical", "3"),
    "7697": ("Reported", "Technical", "5.3"),
}
REQUIREMENTS = {
    "RFC7468-2-BOUNDARIES",
    "RFC7468-2-LABELS",
    "RFC7468-2-BASE64",
    "RFC7468-2-NO-HEADERS",
    "RFC7468-2-64-COLUMNS",
    "RFC7468-2-PARSER-LATITUDE",
    "RFC7468-2-ADJACENT-MULTIPLE",
    "RFC7468-14-LABEL-SEMANTICS",
}


def fail(message: str) -> None:
    raise ValueError(message)


def validate(directory: Path) -> None:
    source = directory / "rfc7468.txt"
    if hashlib.sha256(source.read_bytes()).hexdigest() != EXPECTED_RFC_SHA256:
        fail("rfc7468.txt differs from the locked RFC Editor bytes")
    text = source.read_bytes()
    for marker in (
        b"RFC 7468",
        b"Textual Encodings of PKIX, PKCS, and CMS Structures",
        b"stricttextualmsg",
        b"Generators MUST wrap the base64-encoded lines",
    ):
        if marker not in text:
            fail(f"RFC source is missing marker {marker!r}")

    source_rows = [
        line.split("\t")
        for line in (directory / "SOURCES").read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]
    expected_source = [
        "rfc7468.txt",
        "https://www.rfc-editor.org/rfc/rfc7468.txt",
        EXPECTED_RFC_SHA256,
    ]
    if expected_source not in source_rows or len(source_rows) != 5:
        fail("SOURCES does not contain the locked RFC 7468 HTTPS source")

    with (directory / "rfc7468-errata.tsv").open(
        newline="", encoding="utf-8"
    ) as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    observed = {
        row["id"]: (row["status"], row["type"], row["section"]) for row in rows
    }
    if observed != EXPECTED_ERRATA:
        fail(f"RFC 7468 errata snapshot drifted: {observed!r}")
    for row in rows:
        if row["source"] != f"https://www.rfc-editor.org/errata/eid{row['id']}":
            fail(f"erratum {row['id']} source is not canonical HTTPS")
        if any(not row[field] for field in row):
            fail(f"erratum {row['id']} has incomplete disposition metadata")

    ledger = json.loads(
        (directory / "rfc7468-requirements.json").read_text(encoding="utf-8")
    )
    if ledger.get("schema_version") != 1 or ledger.get("rfc") != 7468:
        fail("RFC 7468 requirements identity drifted")
    requirements = ledger.get("requirements", [])
    identifiers = {entry.get("id") for entry in requirements}
    if identifiers != REQUIREMENTS or len(requirements) != len(REQUIREMENTS):
        fail("RFC 7468 requirement set drifted")
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
        print(f"RFC 7468 policy: {error}", file=sys.stderr)
        return 1
    print("RFC 7468 policy: textual encoding source, errata, and requirements ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
