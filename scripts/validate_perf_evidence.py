#!/usr/bin/env python3
"""Validate and summarize base64-ng performance evidence schema version 1."""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

from perf_evidence_derived import (
    admission_rows,
    grouped_medians,
    summary_rows,
)
from perf_evidence_schema import (
    ADMISSION_FIELDS,
    AVAILABILITY_FIELDS,
    BACKENDS,
    BENCHMARK_FIELDS,
    BINARY_FEATURE_SETS,
    BINARY_RESOURCE_FIELDS,
    COMMIT_ID,
    ENGINES,
    EVIDENCE_ID,
    EXPECTED_LENGTHS,
    OPERATIONS,
    PROFILES,
    RESOURCE_CATEGORIES,
    RESOURCE_FIELDS,
    SUMMARY_FIELDS,
)


def require_evidence_id(value: str, field: str) -> None:
    if not EVIDENCE_ID.fullmatch(value):
        fail(f"{field} must match [A-Za-z0-9][A-Za-z0-9._-]{{0,63}}")


def read_environment(path: Path) -> dict[str, object]:
    try:
        environment = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"{path}: invalid environment JSON: {error}")
    if not isinstance(environment, dict) or environment.get("schema_version") != 1:
        fail(f"{path}: unsupported environment schema")

    source = environment.get("source")
    if not isinstance(source, dict):
        fail(f"{path}: missing source provenance")
    if source.get("status") != "clean":
        fail(f"{path}: performance evidence was not captured from a clean tree")
    commit = source.get("commit")
    if not isinstance(commit, str) or not COMMIT_ID.fullmatch(commit):
        fail(f"{path}: performance evidence has no full source commit")

    measurement = environment.get("measurement")
    if not isinstance(measurement, dict):
        fail(f"{path}: missing measurement contract")
    campaign_id = measurement.get("campaign_id")
    if not isinstance(campaign_id, str):
        fail(f"{path}: missing campaign identifier")
    require_evidence_id(campaign_id, "campaign_id")
    sample_count = measurement.get("sample_count")
    if not isinstance(sample_count, int) or isinstance(sample_count, bool):
        fail(f"{path}: invalid sample count")
    if sample_count < 2:
        fail(f"{path}: sample count must be at least two")
    target_bytes = measurement.get("target_bytes_per_sample")
    if not isinstance(target_bytes, int) or isinstance(target_bytes, bool):
        fail(f"{path}: invalid target bytes")
    if target_bytes <= 0:
        fail(f"{path}: target bytes must be positive")
    return environment


def read_csv(path: Path, fields: list[str]) -> list[dict[str, str]]:
    try:
        with path.open(newline="", encoding="utf-8") as handle:
            reader = csv.reader(handle)
            try:
                header = next(reader)
            except StopIteration:
                fail(f"{path}: empty CSV")
            if header != fields:
                fail(f"{path}: unexpected header {header!r}")
            rows = []
            for line_number, values in enumerate(reader, start=2):
                if len(values) != len(fields):
                    fail(
                        f"{path}:{line_number}: expected {len(fields)} fields, "
                        f"found {len(values)}"
                    )
                rows.append(dict(zip(fields, values, strict=True)))
    except (OSError, csv.Error) as error:
        fail(f"{path}: invalid CSV: {error}")
    if not rows:
        fail(f"{path}: no rows")
    return rows


def validate_benchmark(
    path: Path,
    *,
    availability_rows: list[dict[str, str]],
    environment: dict[str, object],
    expected_run_id: str,
) -> list[dict[str, str]]:
    rows = read_csv(path, BENCHMARK_FIELDS)
    samples: dict[tuple[str, ...], set[int]] = defaultdict(set)

    require_evidence_id(expected_run_id, "expected run_id")

    for row in rows:
        if row["schema_version"] != "1":
            fail(f"{path}: unsupported schema version")
        require_evidence_id(row["campaign_id"], "campaign_id")
        require_evidence_id(row["run_id"], "run_id")
        require_evidence_id(row["target_arch"], "target_arch")
        require_evidence_id(row["target_os"], "target_os")
        profile = (row["alphabet"], row["padding"])
        if profile not in PROFILES:
            fail(f"{path}: unsupported profile {profile!r}")
        if row["operation"] not in OPERATIONS:
            fail(f"{path}: unsupported operation")
        if row["engine"] not in ENGINES:
            fail(f"{path}: unpinned or unknown engine {row['engine']!r}")
        try:
            sample = int(row["sample_index"])
            input_len = int(row["input_len"])
            encoded_len = int(row["encoded_len"])
            iterations = int(row["iterations"])
            elapsed_ns = int(row["elapsed_ns"])
            throughput = float(row["throughput_mib_s"])
            allocations = int(row["allocation_count"])
        except ValueError as error:
            fail(f"{path}: invalid numeric field: {error}")
        if min(input_len, encoded_len, iterations, elapsed_ns) <= 0:
            fail(f"{path}: non-positive benchmark dimension")
        if sample < 0 or not math.isfinite(throughput) or throughput <= 0:
            fail(f"{path}: invalid sample or throughput")
        if allocations != 0:
            fail(f"{path}: slice operation allocated {allocations} times")
        if row["input_len"] not in EXPECTED_LENGTHS:
            fail(f"{path}: unexpected input length {row['input_len']!r}")
        if row["backend"] != "external" and row["backend"] not in BACKENDS:
            fail(f"{path}: unknown backend {row['backend']!r}")
        if row["engine"] == "base64-ng" and row["backend"] == "external":
            fail(f"{path}: base64-ng row uses external backend")
        if row["engine"] != "base64-ng" and row["backend"] != "external":
            fail(f"{path}: comparison engine uses internal backend")
        if row["active_encode_backend"] not in BACKENDS:
            fail(f"{path}: unknown active encode backend")
        if row["active_decode_backend"] not in BACKENDS:
            fail(f"{path}: unknown active decode backend")

        key = tuple(
            row[field]
            for field in (
                "engine",
                "operation",
                "alphabet",
                "padding",
                "input_len",
                "backend",
            )
        )
        if sample in samples[key]:
            fail(f"{path}: duplicate sample for {key!r}")
        samples[key].add(sample)

    measurement = environment.get("measurement")
    if not isinstance(measurement, dict):
        fail("environment measurement contract disappeared")
    campaign_id = measurement.get("campaign_id")
    sample_count = measurement.get("sample_count")
    if not isinstance(campaign_id, str) or not isinstance(sample_count, int):
        fail("environment measurement contract is invalid")

    availability_target = {
        (row["target_arch"], row["target_os"]) for row in availability_rows
    }
    if len(availability_target) != 1:
        fail("availability does not identify exactly one target")
    target_arch, target_os = next(iter(availability_target))
    available_ng = {
        row["backend"] for row in availability_rows if row["available"] == "true"
    }
    expected_groups = {
        (
            "base64-ng",
            operation,
            alphabet,
            padding,
            length,
            backend,
        )
        for operation in OPERATIONS
        for alphabet, padding in PROFILES
        for length in EXPECTED_LENGTHS
        for backend in available_ng
    }
    expected_groups |= {
        (engine, operation, alphabet, padding, length, "external")
        for engine in ENGINES - {"base64-ng"}
        for operation in OPERATIONS
        for alphabet, padding in PROFILES
        for length in EXPECTED_LENGTHS
    }
    observed_groups = set(samples)
    if observed_groups != expected_groups:
        missing = sorted(expected_groups - observed_groups)
        extra = sorted(observed_groups - expected_groups)
        fail(f"{path}: incomplete matrix: missing={missing[:10]}, extra={extra[:10]}")

    expected_samples = set(range(sample_count))
    for key, observed_samples in samples.items():
        if observed_samples != expected_samples:
            fail(
                f"{path}: sample indexes for {key!r} are {sorted(observed_samples)!r}; "
                f"expected {sorted(expected_samples)!r}"
            )
    for row in rows:
        if row["campaign_id"] != campaign_id:
            fail(f"{path}: campaign identifier does not match environment")
        if row["run_id"] != expected_run_id:
            fail(f"{path}: unexpected run identifier")
        if (row["target_arch"], row["target_os"]) != (target_arch, target_os):
            fail(f"{path}: benchmark target does not match availability")
    return rows


def validate_auxiliary(
    availability: Path, resources: Path, expected_feature_set: str
) -> list[dict[str, str]]:
    require_evidence_id(expected_feature_set, "expected feature_set")
    availability_rows = read_csv(availability, AVAILABILITY_FIELDS)
    names = {row["backend"] for row in availability_rows}
    if names != BACKENDS or len(availability_rows) != len(BACKENDS):
        fail(f"{availability}: incomplete backend inventory")
    for row in availability_rows:
        if row["schema_version"] != "1":
            fail(f"{availability}: unsupported schema version")
        if row["available"] not in {"true", "false"}:
            fail(f"{availability}: invalid availability value")
        if row["backend"] not in BACKENDS:
            fail(f"{availability}: invalid backend")
        require_evidence_id(row["backend"], "backend")
        require_evidence_id(row["target_arch"], "target_arch")
        require_evidence_id(row["target_os"], "target_os")
    if len(
        {(row["target_arch"], row["target_os"]) for row in availability_rows}
    ) != 1:
        fail(f"{availability}: inconsistent target")
    available = {
        row["backend"] for row in availability_rows if row["available"] == "true"
    }
    if not {"auto", "scalar"} <= available:
        fail(f"{availability}: auto and scalar must be available")

    resource_rows = read_csv(resources, RESOURCE_FIELDS)
    categories = {row["category"] for row in resource_rows}
    if not RESOURCE_CATEGORIES <= categories:
        fail(f"{resources}: incomplete resource categories")
    resource_keys = {
        (row["category"], row["name"], row["feature_set"]) for row in resource_rows
    }
    if len(resource_keys) != len(resource_rows):
        fail(f"{resources}: duplicate resource row")
    for row in resource_rows:
        if row["schema_version"] != "1":
            fail(f"{resources}: unsupported schema version")
        for field in ("category", "name", "feature_set", "unit", "method"):
            require_evidence_id(row[field], field)
        if row["category"] not in RESOURCE_CATEGORIES:
            fail(f"{resources}: unsupported resource category")
        if row["feature_set"] != expected_feature_set:
            fail(f"{resources}: unexpected feature set")
        try:
            value = int(row["value"])
        except ValueError as error:
            fail(f"{resources}: invalid resource value: {error}")
        if value < 0:
            fail(f"{resources}: negative resource value")
    return availability_rows


def load_validated_bundle(
    directory: Path,
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    environment = read_environment(directory / "environment.json")
    availability = directory / "availability.csv"
    first_availability = validate_auxiliary(
        availability, directory / "resources-default.csv", "simd"
    )
    second_availability = validate_auxiliary(
        availability, directory / "resources-no-simd.csv", "no-simd"
    )
    first_rows = validate_benchmark(
        directory / "raw-run-1.csv",
        availability_rows=first_availability,
        environment=environment,
        expected_run_id="run-1",
    )
    second_rows = validate_benchmark(
        directory / "raw-run-2.csv",
        availability_rows=second_availability,
        environment=environment,
        expected_run_id="run-2",
    )
    return first_rows, second_rows


def compare(directory: Path, lower: float, upper: float) -> None:
    first_rows, second_rows = load_validated_bundle(directory)
    first_medians = grouped_medians(first_rows)
    second_medians = grouped_medians(second_rows)
    if first_medians.keys() != second_medians.keys():
        fail("benchmark runs do not contain the same measurement matrix")
    failures = []
    for key in sorted(first_medians):
        ratio = second_medians[key] / first_medians[key]
        if not lower <= ratio <= upper:
            failures.append((key, ratio))
    if failures:
        fail(
            "same-host reproducibility threshold exceeded: "
            + "; ".join(f"{key}={ratio:.3f}" for key, ratio in failures[:10])
        )
    print(
        f"performance evidence: reproducible within [{lower:.2f}, {upper:.2f}] "
        f"for {len(first_medians)} measurement groups"
    )


def write_rows(fields: list[str], rows: list[list[str]]) -> None:
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(fields)
    writer.writerows(rows)


def summarize(directory: Path) -> None:
    first_rows, second_rows = load_validated_bundle(directory)
    write_rows(SUMMARY_FIELDS, summary_rows(first_rows, second_rows))


def admission(directory: Path, minimum_ratio: float) -> None:
    first_rows, second_rows = load_validated_bundle(directory)
    write_rows(
        ADMISSION_FIELDS,
        admission_rows(first_rows, second_rows, minimum_ratio),
    )


def row_values(rows: list[dict[str, str]], fields: list[str]) -> list[list[str]]:
    return [[row[field] for field in fields] for row in rows]


def validate_binary_resources(path: Path) -> None:
    rows = read_csv(path, BINARY_RESOURCE_FIELDS)
    feature_sets = {row["feature_set"] for row in rows}
    if feature_sets != BINARY_FEATURE_SETS or len(rows) != len(BINARY_FEATURE_SETS):
        fail(f"{path}: incomplete binary feature-set inventory")
    for row in rows:
        if row["schema_version"] != "1":
            fail(f"{path}: unsupported schema version")
        require_evidence_id(row["feature_set"], "feature_set")
        require_evidence_id(row["method"], "method")
        if row["method"] != "nm-and-file-size":
            fail(f"{path}: unsupported measurement method")
        try:
            binary_bytes = int(row["binary_bytes"])
            symbol_count = int(row["base64_ng_symbol_count"])
        except ValueError as error:
            fail(f"{path}: invalid numeric field: {error}")
        if binary_bytes <= 0 or symbol_count < 0:
            fail(f"{path}: invalid binary resource value")


def validate_derived(directory: Path) -> None:
    first_rows, second_rows = load_validated_bundle(directory)
    summary = read_csv(directory / "summary.csv", SUMMARY_FIELDS)
    expected_summary = summary_rows(first_rows, second_rows)
    if row_values(summary, SUMMARY_FIELDS) != expected_summary:
        fail(f"{directory / 'summary.csv'}: does not match raw evidence")
    admitted = read_csv(directory / "admission.csv", ADMISSION_FIELDS)
    expected_admission = admission_rows(first_rows, second_rows, 0.95)
    if row_values(admitted, ADMISSION_FIELDS) != expected_admission:
        fail(f"{directory / 'admission.csv'}: does not match raw evidence")
    validate_binary_resources(directory / "binary-resources.csv")


def fail(message: str) -> None:
    raise SystemExit(f"performance evidence: {message}")


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser("validate")
    validate_parser.add_argument("benchmark", type=Path)
    validate_parser.add_argument("availability", type=Path)
    validate_parser.add_argument("resources", type=Path)
    validate_parser.add_argument("environment", type=Path)
    validate_parser.add_argument("--expected-run-id", required=True)
    validate_parser.add_argument("--expected-feature-set", required=True)

    compare_parser = subparsers.add_parser("compare")
    compare_parser.add_argument("directory", type=Path)
    compare_parser.add_argument("--lower", type=float, default=0.50)
    compare_parser.add_argument("--upper", type=float, default=2.00)

    summarize_parser = subparsers.add_parser("summarize")
    summarize_parser.add_argument("directory", type=Path)

    admission_parser = subparsers.add_parser("admission")
    admission_parser.add_argument("directory", type=Path)
    admission_parser.add_argument("--minimum-ratio", type=float, default=0.95)

    derived_parser = subparsers.add_parser("validate-derived")
    derived_parser.add_argument("directory", type=Path)

    args = parser.parse_args()
    if args.command == "validate":
        environment = read_environment(args.environment)
        availability_rows = validate_auxiliary(
            args.availability, args.resources, args.expected_feature_set
        )
        validate_benchmark(
            args.benchmark,
            availability_rows=availability_rows,
            environment=environment,
            expected_run_id=args.expected_run_id,
        )
        print("performance evidence: schema and matrix ok")
    elif args.command == "compare":
        compare(args.directory, args.lower, args.upper)
    elif args.command == "summarize":
        summarize(args.directory)
    elif args.command == "admission":
        admission(args.directory, args.minimum_ratio)
    elif args.command == "validate-derived":
        validate_derived(args.directory)
        print("performance evidence: derived artifacts ok")


if __name__ == "__main__":
    main()
