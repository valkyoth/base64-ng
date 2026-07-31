#!/usr/bin/env python3
"""Validate and summarize base64-ng performance evidence schema version 1."""

from __future__ import annotations

import argparse
import csv
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path

BENCHMARK_FIELDS = [
    "schema_version",
    "campaign_id",
    "run_id",
    "sample_index",
    "engine",
    "operation",
    "alphabet",
    "padding",
    "input_len",
    "encoded_len",
    "iterations",
    "elapsed_ns",
    "throughput_mib_s",
    "backend",
    "active_encode_backend",
    "active_decode_backend",
    "target_arch",
    "target_os",
    "allocation_count",
]
AVAILABILITY_FIELDS = [
    "schema_version",
    "backend",
    "available",
    "target_arch",
    "target_os",
]
RESOURCE_FIELDS = [
    "schema_version",
    "category",
    "name",
    "feature_set",
    "value",
    "unit",
    "method",
]
PROFILES = {
    ("standard", "padded"),
    ("standard", "unpadded"),
    ("url-safe", "padded"),
    ("url-safe", "unpadded"),
}
OPERATIONS = {"encode", "decode"}
ENGINES = {"base64-ng", "base64-0.23.0", "base64ct-1.8.3"}
BACKEND_MINIMUM = {
    "auto": {"encode": 1, "decode": 1},
    "ssse3-sse4.1": {"encode": 12, "decode": 12},
    "avx2": {"encode": 24, "decode": 24},
    "avx512-vbmi": {"encode": 48, "decode": 48},
    "neon": {"encode": 12, "decode": 12},
    "wasm-simd128": {"encode": 12, "decode": 12},
}


def read_csv(path: Path, fields: list[str]) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames != fields:
            fail(f"{path}: unexpected header {reader.fieldnames!r}")
        rows = list(reader)
    if not rows:
        fail(f"{path}: no rows")
    return rows


def validate_benchmark(path: Path) -> list[dict[str, str]]:
    rows = read_csv(path, BENCHMARK_FIELDS)
    seen_profiles: set[tuple[str, str]] = set()
    seen_operations: set[str] = set()
    seen_engines: set[str] = set()
    samples: dict[tuple[str, ...], set[int]] = defaultdict(set)

    for row in rows:
        if row["schema_version"] != "1":
            fail(f"{path}: unsupported schema version")
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
        seen_profiles.add(profile)
        seen_operations.add(row["operation"])
        seen_engines.add(row["engine"])

    if seen_profiles != PROFILES:
        fail(f"{path}: incomplete profile matrix")
    if seen_operations != OPERATIONS or seen_engines != ENGINES:
        fail(f"{path}: incomplete engine or operation matrix")
    sample_counts = {len(value) for value in samples.values()}
    if len(sample_counts) != 1 or min(sample_counts) < 2:
        fail(f"{path}: inconsistent or insufficient sample counts")
    return rows


def validate_auxiliary(availability: Path, resources: Path) -> None:
    availability_rows = read_csv(availability, AVAILABILITY_FIELDS)
    names = {row["backend"] for row in availability_rows}
    required = {
        "auto",
        "scalar",
        "ssse3-sse4.1",
        "avx2",
        "avx512-vbmi",
        "neon",
        "wasm-simd128",
    }
    if names != required:
        fail(f"{availability}: incomplete backend inventory")
    if any(row["available"] not in {"true", "false"} for row in availability_rows):
        fail(f"{availability}: invalid availability value")

    resource_rows = read_csv(resources, RESOURCE_FIELDS)
    categories = {row["category"] for row in resource_rows}
    if not {"stack-bound", "adapter-pending-memory", "adapter-size"} <= categories:
        fail(f"{resources}: incomplete resource categories")
    for row in resource_rows:
        if int(row["value"]) < 0:
            fail(f"{resources}: negative resource value")


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
        validate_benchmark(args.benchmark)
        validate_auxiliary(args.availability, args.resources)
        print("performance evidence: schema and matrix ok")
    elif args.command == "compare":
        compare(args.first, args.second, args.lower, args.upper)
    elif args.command == "summarize":
        summarize(args.first, args.second)
    elif args.command == "admission":
        admission(args.first, args.second, args.minimum_ratio)


if __name__ == "__main__":
    main()
