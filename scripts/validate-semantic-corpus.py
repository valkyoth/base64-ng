#!/usr/bin/env python3
"""Validate the versioned cross-crate semantic corpus schema."""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path


ROOT = Path("semantic-corpus/v1")
EXPECTED_COLUMNS = [
    "id",
    "profile",
    "operation",
    "input_hex",
    "encoded",
    "decision",
    "error_class",
    "error_offset",
    "eof",
    "partitions",
    "committed_prefix_hex",
    "core_one_shot",
    "core_stream",
    "bytes",
    "tokio",
    "serde",
    "sanitization",
]
PROFILES = {
    "standard-pad",
    "standard-no-pad",
    "url-safe-pad",
    "url-safe-no-pad",
}
CONTRACTS = {
    "byte-identical",
    "atomic-unchanged",
    "committed-prefix",
    "irrevocable-sink-progress",
    "opaque-reject",
    "not-applicable",
}


def fail(message: str) -> None:
    raise ValueError(message)


def main() -> int:
    try:
        schema = json.loads((ROOT / "schema.json").read_text(encoding="utf-8"))
        if schema.get("schema_version") != 1:
            fail("schema version must be 1")
        if list(schema.get("columns", {})) != EXPECTED_COLUMNS:
            fail("schema columns do not match the locked TSV order")
        if not schema.get("failure_contract") or not schema.get("secret_error_policy"):
            fail("schema omits failure or secret error policy")

        with (ROOT / "cases.tsv").open(newline="", encoding="utf-8") as handle:
            reader = csv.DictReader(handle, delimiter="\t")
            if reader.fieldnames != EXPECTED_COLUMNS:
                fail("case header does not match schema")
            rows = list(reader)
        if not rows:
            fail("corpus has no cases")

        identifiers: set[str] = set()
        for row in rows:
            identifier = row["id"]
            if not identifier or identifier in identifiers:
                fail(f"invalid or duplicate case id {identifier!r}")
            identifiers.add(identifier)
            if row["profile"] not in PROFILES:
                fail(f"{identifier}: unknown profile")
            if row["operation"] not in {"round-trip", "decode-error"}:
                fail(f"{identifier}: unknown operation")
            bytes.fromhex(row["input_hex"])
            row["encoded"].encode("ascii")
            if row["decision"] not in {"canonical", "reject"}:
                fail(f"{identifier}: unknown decision")
            if row["eof"] not in {"complete", "malformed", "incomplete"}:
                fail(f"{identifier}: unknown EOF state")
            partitions = [int(value) for value in row["partitions"].split(",")]
            if any(value < 0 for value in partitions):
                fail(f"{identifier}: negative partition")
            bytes.fromhex(row["committed_prefix_hex"])
            for surface in EXPECTED_COLUMNS[11:]:
                if row[surface] not in CONTRACTS:
                    fail(f"{identifier}: unknown {surface} contract")
            if row["operation"] == "round-trip":
                if row["decision"] != "canonical" or row["error_class"] != "-":
                    fail(f"{identifier}: successful row has error policy")
                for surface in ("core_one_shot", "core_stream", "bytes", "tokio"):
                    if row[surface] != "byte-identical":
                        fail(f"{identifier}: successful {surface} is not byte-identical")
                for surface in ("serde", "sanitization"):
                    if row[surface] not in {"byte-identical", "not-applicable"}:
                        fail(f"{identifier}: invalid successful {surface} contract")
            elif row["decision"] != "reject" or row["error_class"] == "-":
                fail(f"{identifier}: rejection row lacks normalized error")
            else:
                if row["core_one_shot"] not in {
                    "atomic-unchanged",
                    "committed-prefix",
                }:
                    fail(f"{identifier}: invalid core one-shot failure contract")
                if row["core_stream"] != "irrevocable-sink-progress":
                    fail(f"{identifier}: invalid core stream failure contract")
                for surface in ("bytes", "tokio"):
                    if row[surface] != "atomic-unchanged":
                        fail(f"{identifier}: invalid {surface} failure contract")
                if row["serde"] not in {"atomic-unchanged", "not-applicable"}:
                    fail(f"{identifier}: invalid serde failure contract")
                if row["sanitization"] not in {"opaque-reject", "not-applicable"}:
                    fail(f"{identifier}: invalid sanitization failure contract")
        print(f"semantic corpus: schema and {len(rows)} cases ok")
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"semantic corpus: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
