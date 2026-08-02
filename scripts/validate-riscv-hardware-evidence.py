#!/usr/bin/env python3
"""Validate the dependency-free real-RVV evidence contract."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "hardware-evidence/riscv/schema-v1.json"
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
RANGE = re.compile(r"^([0-9a-f]{40})\.\.([0-9a-f]{40})$")


def fail(message: str) -> None:
    raise ValueError(message)


def exact_object(value: Any, path: str, fields: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(f"{path} must be an object")
    missing = fields - value.keys()
    extra = value.keys() - fields
    if missing or extra:
        fail(f"{path} fields differ: missing={sorted(missing)} extra={sorted(extra)}")
    return value


def text(value: Any, path: str, maximum: int = 256) -> str:
    if not isinstance(value, str) or not value or len(value) > maximum:
        fail(f"{path} must be a non-empty string of at most {maximum} characters")
    return value


ROOT_FIELDS = {
    "schema_version", "project", "execution_environment", "source_commit",
    "target", "hardware", "software", "vector", "verification", "backend",
    "benchmark", "review",
}


def validate_schema() -> None:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        fail("schema must use JSON Schema draft 2020-12")
    if set(schema.get("required", [])) != ROOT_FIELDS:
        fail("schema required fields drifted from the validator")
    if schema.get("additionalProperties") is not False:
        fail("schema root must reject additional properties")


def validate_report(path: Path) -> None:
    report = exact_object(json.loads(path.read_text(encoding="utf-8")), "report", ROOT_FIELDS)
    if report["schema_version"] != 1 or report["project"] != "base64-ng":
        fail("schema_version/project mismatch")
    if report["execution_environment"] != "real-hardware":
        fail("QEMU, emulators, and virtual machines are not hardware evidence")
    source = text(report["source_commit"], "source_commit", 40)
    if HEX40.fullmatch(source) is None:
        fail("source_commit must be a lowercase 40-character Git object id")

    target = exact_object(report["target"], "target", {"triple", "arch", "endian"})
    if target != {"triple": "riscv64gc-unknown-linux-gnu", "arch": "riscv64", "endian": "little"}:
        fail("target must be the reviewed riscv64gc Linux target")

    hardware = exact_object(report["hardware"], "hardware", {"vendor", "board", "soc", "cpu", "firmware"})
    for field in hardware:
        text(hardware[field], f"hardware.{field}")
    software = exact_object(report["software"], "software", {"os", "kernel", "rustc", "cargo"})
    for field in software:
        text(software[field], f"software.{field}")

    vector = exact_object(report["vector"], "vector", {"specification", "vlen_bits", "hwprobe_v", "vector_state_on", "signal_context_review", "ffi_abi_review"})
    if vector["specification"] != "1.0" or vector["hwprobe_v"] is not True or vector["vector_state_on"] is not True:
        fail("RVV 1.0 hardware and enabled vector state are required")
    vlen = vector["vlen_bits"]
    if isinstance(vlen, bool) or not isinstance(vlen, int) or not 128 <= vlen <= 65536 or vlen % 8:
        fail("vector.vlen_bits must be an 8-bit multiple in 128..=65536")
    if vector["signal_context_review"] != "PASS" or vector["ffi_abi_review"] != "PASS":
        fail("signal/context and FFI ABI reviews must PASS")

    verification = exact_object(report["verification"], "verification", {"command", "passed", "output_sha256"})
    if verification["command"] != "scripts/check_riscv_hardware.sh" or verification["passed"] is not True:
        fail("the native hardware gate must pass")
    if not isinstance(verification["output_sha256"], str) or HEX64.fullmatch(verification["output_sha256"]) is None:
        fail("verification.output_sha256 must be a lowercase SHA-256")

    backend = exact_object(report["backend"], "backend", {"encode", "strict_decode", "secret_decode", "production_admitted"})
    expected_backend = {"encode": "rvv-candidate", "strict_decode": "rvv-candidate", "secret_decode": "scalar-constant-time-oriented", "production_admitted": False}
    if backend != expected_backend:
        fail("Commit 32 evidence must remain candidate-only and keep secret decode scalar")

    benchmark = exact_object(report["benchmark"], "benchmark", {"command", "encode_beneficial", "decode_beneficial", "raw_data_sha256"})
    text(benchmark["command"], "benchmark.command", 1024)
    if benchmark["encode_beneficial"] is not True or benchmark["decode_beneficial"] is not True:
        fail("real hardware must show both candidate operations are beneficial")
    if not isinstance(benchmark["raw_data_sha256"], str) or HEX64.fullmatch(benchmark["raw_data_sha256"]) is None:
        fail("benchmark.raw_data_sha256 must be a lowercase SHA-256")

    review = exact_object(report["review"], "review", {"reporter", "recorded_at", "assembly", "register_cleanup", "pentest_range", "pentest_result"})
    text(review["reporter"], "review.reporter")
    recorded = text(review["recorded_at"], "review.recorded_at", 64)
    try:
        parsed = dt.datetime.fromisoformat(recorded.replace("Z", "+00:00"))
    except ValueError as error:
        fail(f"review.recorded_at must be ISO-8601: {error}")
    if parsed.tzinfo is None:
        fail("review.recorded_at must include a timezone")
    match = RANGE.fullmatch(text(review["pentest_range"], "review.pentest_range", 82))
    if match is None or match.group(2) != source:
        fail("review.pentest_range must end at source_commit")
    if review["assembly"] != "PASS" or review["register_cleanup"] != "PASS" or review["pentest_result"] != "PASS":
        fail("assembly, cleanup, and pentest reviews must PASS")


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
        print(f"RISC-V hardware evidence: {error}", file=sys.stderr)
        return 1
    print("RISC-V hardware evidence: schema and report ok" if args.report else "RISC-V hardware evidence: schema ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
