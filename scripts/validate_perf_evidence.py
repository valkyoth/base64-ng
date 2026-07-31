#!/usr/bin/env python3
"""Validate and summarize base64-ng performance evidence schema version 1."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path

from perf_evidence_schema import (
    AVAILABILITY_FIELDS,
    BACKEND_MINIMUM,
    BACKENDS,
    BENCHMARK_FIELDS,
    COMMIT_ID,
    ENGINES,
    EVIDENCE_ID,
    EXPECTED_LENGTHS,
    OPERATIONS,
    PROFILES,
    RESOURCE_FIELDS,
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
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames != fields:
            fail(f"{path}: unexpected header {reader.fieldnames!r}")
        rows = list(reader)
    if not rows:
        fail(f"{path}: no rows")
    return rows


def validate_benchmark(
    path: Path,
    *,
    availability_rows: list[dict[str, str]] | None = None,
    environment: dict[str, object] | None = None,
    expected_run_id: str | None = None,
) -> list[dict[str, str]]:
    rows = read_csv(path, BENCHMARK_FIELDS)
    samples: dict[tuple[str, ...], set[int]] = defaultdict(set)

    if expected_run_id is not None:
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

    if availability_rows is None or environment is None or expected_run_id is None:
        sample_counts = {len(value) for value in samples.values()}
        if len(sample_counts) != 1 or min(sample_counts) < 2:
            fail(f"{path}: inconsistent or insufficient sample counts")
        return rows

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
    if any(row["available"] not in {"true", "false"} for row in availability_rows):
        fail(f"{availability}: invalid availability value")
    if any(row["backend"] not in BACKENDS for row in availability_rows):
        fail(f"{availability}: invalid backend")
    if any(
        not EVIDENCE_ID.fullmatch(row["target_arch"])
        or not EVIDENCE_ID.fullmatch(row["target_os"])
        for row in availability_rows
    ):
        fail(f"{availability}: invalid target identifier")
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
    if not {"stack-bound", "adapter-pending-memory", "adapter-size"} <= categories:
        fail(f"{resources}: incomplete resource categories")
    for row in resource_rows:
        require_evidence_id(row["feature_set"], "feature_set")
        if row["feature_set"] != expected_feature_set:
            fail(f"{resources}: unexpected feature set")
        if int(row["value"]) < 0:
            fail(f"{resources}: negative resource value")
    return availability_rows


def measurement_key(row: dict[str, str]) -> tuple[str, ...]:
    return tuple(
        row[field]
        for field in (
            "engine",
            "operation",
            "alphabet",
            "padding",
            "input_len",
            "backend",
            "target_arch",
            "target_os",
        )
    )


def grouped_medians(rows: list[dict[str, str]]) -> dict[tuple[str, ...], float]:
    grouped: dict[tuple[str, ...], list[float]] = defaultdict(list)
    for row in rows:
        grouped[measurement_key(row)].append(float(row["throughput_mib_s"]))
    return {key: statistics.median(values) for key, values in grouped.items()}


def compare(first: Path, second: Path, lower: float, upper: float) -> None:
    first_rows = validate_benchmark(first)
    second_rows = validate_benchmark(second)
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


def summarize(first: Path, second: Path) -> None:
    rows = validate_benchmark(first) + validate_benchmark(second)
    grouped: dict[tuple[str, ...], list[float]] = defaultdict(list)
    for row in rows:
        grouped[measurement_key(row)].append(float(row["throughput_mib_s"]))
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(
        [
            "schema_version",
            "engine",
            "operation",
            "alphabet",
            "padding",
            "input_len",
            "backend",
            "target_arch",
            "target_os",
            "sample_count",
            "median_throughput_mib_s",
            "minimum_throughput_mib_s",
            "maximum_throughput_mib_s",
        ]
    )
    for key in sorted(grouped):
        values = grouped[key]
        writer.writerow(
            [
                "1",
                *key,
                len(values),
                f"{statistics.median(values):.6f}",
                f"{min(values):.6f}",
                f"{max(values):.6f}",
            ]
        )


def admission(first: Path, second: Path, minimum_ratio: float) -> None:
    rows = validate_benchmark(first) + validate_benchmark(second)
    medians = grouped_medians(rows)
    scalar: dict[tuple[str, ...], float] = {}
    for key, value in medians.items():
        engine, operation, alphabet, padding, length, backend, arch, os_name = key
        if engine == "base64-ng" and backend == "scalar":
            scalar[(operation, alphabet, padding, length, arch, os_name)] = value

    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(
        [
            "schema_version",
            "backend",
            "operation",
            "alphabet",
            "padding",
            "input_len",
            "target_arch",
            "target_os",
            "median_throughput_mib_s",
            "scalar_median_throughput_mib_s",
            "ratio_to_scalar",
            "status",
        ]
    )
    for key, value in sorted(medians.items()):
        engine, operation, alphabet, padding, length, backend, arch, os_name = key
        if engine != "base64-ng" or backend == "scalar":
            continue
        minimum = BACKEND_MINIMUM.get(backend, {}).get(operation)
        if minimum is None or int(length) < minimum:
            continue
        scalar_value = scalar[(operation, alphabet, padding, length, arch, os_name)]
        ratio = value / scalar_value
        status = (
            "admissible"
            if ratio >= minimum_ratio
            else "non-admissible-below-scalar"
        )
        writer.writerow(
            [
                "1",
                backend,
                operation,
                alphabet,
                padding,
                length,
                arch,
                os_name,
                f"{value:.6f}",
                f"{scalar_value:.6f}",
                f"{ratio:.6f}",
                status,
            ]
        )


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
    compare_parser.add_argument("first", type=Path)
    compare_parser.add_argument("second", type=Path)
    compare_parser.add_argument("--lower", type=float, default=0.50)
    compare_parser.add_argument("--upper", type=float, default=2.00)

    summarize_parser = subparsers.add_parser("summarize")
    summarize_parser.add_argument("first", type=Path)
    summarize_parser.add_argument("second", type=Path)

    admission_parser = subparsers.add_parser("admission")
    admission_parser.add_argument("first", type=Path)
    admission_parser.add_argument("second", type=Path)
    admission_parser.add_argument("--minimum-ratio", type=float, default=0.95)

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
        compare(args.first, args.second, args.lower, args.upper)
    elif args.command == "summarize":
        summarize(args.first, args.second)
    elif args.command == "admission":
        admission(args.first, args.second, args.minimum_ratio)


if __name__ == "__main__":
    main()
