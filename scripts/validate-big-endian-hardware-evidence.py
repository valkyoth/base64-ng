#!/usr/bin/env python3
"""Validate the dependency-free big-endian community evidence contract."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "hardware-evidence/big-endian/schema-v1.json"
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
RANGE = re.compile(r"^([0-9a-f]{40})\.\.([0-9a-f]{40})$")
TARGETS = {
    "s390x-unknown-linux-gnu",
    "powerpc64-unknown-linux-gnu",
    "aarch64_be-unknown-linux-gnu",
}


def fail(message: str) -> None:
    raise ValueError(message)


def object_with_keys(value: Any, path: str, required: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(f"{path} must be an object")
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        fail(f"{path} is missing: {', '.join(sorted(missing))}")
    if extra:
        fail(f"{path} contains unknown fields: {', '.join(sorted(extra))}")
    return value


def bounded_string(value: Any, path: str, maximum: int) -> str:
    if not isinstance(value, str) or not value or len(value) > maximum:
        fail(f"{path} must be a non-empty string of at most {maximum} characters")
    return value


def validate_schema() -> None:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        fail("schema must use JSON Schema draft 2020-12")
    required = set(schema.get("required", []))
    expected = {
        "schema_version",
        "project",
        "execution_environment",
        "source_commit",
        "target",
        "hardware",
        "software",
        "verification",
        "backend",
        "review",
    }
    if required != expected or schema.get("additionalProperties") is not False:
        fail("schema root contract drifted from the dependency-free validator")


def validate_report(path: Path) -> None:
    value = json.loads(path.read_text(encoding="utf-8"))
    report = object_with_keys(
        value,
        "report",
        {
            "schema_version",
            "project",
            "execution_environment",
            "source_commit",
            "target",
            "hardware",
            "software",
            "verification",
            "backend",
            "review",
        },
    )
    if report["schema_version"] != 1:
        fail("schema_version must be 1")
    if report["project"] != "base64-ng":
        fail("project must be base64-ng")
    if report["execution_environment"] != "real-hardware":
        fail("execution_environment must be real-hardware; QEMU is not accepted")
    source_commit = bounded_string(report["source_commit"], "source_commit", 40)
    if HEX40.fullmatch(source_commit) is None:
        fail("source_commit must be one lowercase 40-character Git object id")

    target = object_with_keys(report["target"], "target", {"triple", "endian"})
    if target["triple"] not in TARGETS or target["endian"] != "big":
        fail("target must be one admitted big-endian evidence triple")

    hardware = object_with_keys(
        report["hardware"],
        "hardware",
        {"vendor", "model", "cpu", "features", "firmware"},
    )
    for field, maximum in {
        "vendor": 128,
        "model": 128,
        "cpu": 256,
        "features": 4096,
        "firmware": 256,
    }.items():
        bounded_string(hardware[field], f"hardware.{field}", maximum)

    software = object_with_keys(
        report["software"], "software", {"os", "kernel", "rustc", "cargo"}
    )
    for field in software:
        bounded_string(software[field], f"software.{field}", 256)

    verification = object_with_keys(
        report["verification"],
        "verification",
        {"command", "passed", "output_sha256"},
    )
    if verification["command"] != "scripts/check_big_endian_hardware.sh":
        fail("verification.command must name the reviewed hardware gate")
    if verification["passed"] is not True:
        fail("verification.passed must be true")
    if not isinstance(verification["output_sha256"], str) or HEX64.fullmatch(
        verification["output_sha256"]
    ) is None:
        fail("verification.output_sha256 must be one lowercase SHA-256")

    backend = object_with_keys(
        report["backend"],
        "backend",
        {"encode", "strict_decode", "secret_decode", "accelerated"},
    )
    bounded_string(backend["encode"], "backend.encode", 64)
    bounded_string(backend["strict_decode"], "backend.strict_decode", 64)
    if backend["secret_decode"] != "scalar-constant-time-oriented":
        fail("secret_decode must remain scalar-constant-time-oriented")
    if backend["accelerated"] is not False:
        fail("Commit 31 accepts scalar hardware evidence only")
    if backend["encode"] != "scalar" or backend["strict_decode"] != "scalar":
        fail("Commit 31 reports must record scalar ordinary backends")

    review = object_with_keys(
        report["review"],
        "review",
        {"reporter", "recorded_at", "pentest_range", "pentest_result"},
    )
    bounded_string(review["reporter"], "review.reporter", 256)
    recorded_at = bounded_string(review["recorded_at"], "review.recorded_at", 64)
    try:
        parsed = dt.datetime.fromisoformat(recorded_at.replace("Z", "+00:00"))
    except ValueError as error:
        fail(f"review.recorded_at must be an ISO-8601 date-time: {error}")
    if parsed.tzinfo is None:
        fail("review.recorded_at must include a timezone")
    match = RANGE.fullmatch(bounded_string(review["pentest_range"], "review.pentest_range", 82))
    if match is None or match.group(2) != source_commit:
        fail("review.pentest_range must end at source_commit")
    if review["pentest_result"] != "PASS":
        fail("review.pentest_result must be PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", nargs="?", type=Path)
    parser.add_argument("--schema-only", action="store_true")
    args = parser.parse_args()
    try:
        validate_schema()
        if not args.schema_only:
            if args.report is None:
                parser.error("REPORT is required unless --schema-only is used")
            validate_report(args.report)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"big-endian hardware evidence: {error}", file=sys.stderr)
        return 1
    print("big-endian hardware evidence: schema and report ok" if args.report else "big-endian hardware evidence: schema ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
