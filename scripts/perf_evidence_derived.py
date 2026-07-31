"""Deterministic derived rows for performance evidence."""

from __future__ import annotations

import statistics
from collections import defaultdict

from perf_evidence_schema import BACKEND_MINIMUM


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


def grouped_medians(
    rows: list[dict[str, str]],
) -> dict[tuple[str, ...], float]:
    grouped: dict[tuple[str, ...], list[float]] = defaultdict(list)
    for row in rows:
        grouped[measurement_key(row)].append(float(row["throughput_mib_s"]))
    return {key: statistics.median(values) for key, values in grouped.items()}


def summary_rows(
    first_rows: list[dict[str, str]],
    second_rows: list[dict[str, str]],
) -> list[list[str]]:
    grouped: dict[tuple[str, ...], list[float]] = defaultdict(list)
    for row in first_rows + second_rows:
        grouped[measurement_key(row)].append(float(row["throughput_mib_s"]))
    result = []
    for key in sorted(grouped):
        values = grouped[key]
        result.append(
            [
                "1",
                *key,
                str(len(values)),
                f"{statistics.median(values):.6f}",
                f"{min(values):.6f}",
                f"{max(values):.6f}",
            ]
        )
    return result


def admission_rows(
    first_rows: list[dict[str, str]],
    second_rows: list[dict[str, str]],
    minimum_ratio: float,
) -> list[list[str]]:
    medians = grouped_medians(first_rows + second_rows)
    scalar: dict[tuple[str, ...], float] = {}
    for key, value in medians.items():
        engine, operation, alphabet, padding, length, backend, arch, os_name = key
        if engine == "base64-ng" and backend == "scalar":
            scalar[(operation, alphabet, padding, length, arch, os_name)] = value

    result = []
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
        result.append(
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
    return result
