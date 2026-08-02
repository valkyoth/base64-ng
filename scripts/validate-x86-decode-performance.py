#!/usr/bin/env python3
"""Validate Commit 28 exact-backend strict-decode performance evidence."""

import csv
import math
import os
import sys
from collections import defaultdict
from pathlib import Path

from performance_statistics import MINIMUM_SAMPLES, validate_advantage


def fail(message: str) -> None:
    raise SystemExit(f"x86 decode performance: {message}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-x86-decode-performance.py <evidence.csv>")
    path = Path(sys.argv[1])
    minimum_ratio = float(os.environ.get("BASE64_NG_X86_DECODE_RATIO", "1.02"))
    if minimum_ratio < 1.02:
        fail("environment cannot weaken the frozen 1.02 admission ratio")
    samples: dict[tuple[str, str, str, int], list[float]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            key = (
                row["backend"],
                row["alphabet"],
                row["padding"],
                int(row["input_len"]),
            )
            samples[key].append(float(row["throughput_mib_s"]))
    required_lengths = {
        "ssse3-sse4.1": (12, 24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
        "avx2": (24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
    }
    for backend, lengths in required_lengths.items():
        for alphabet in ("standard", "url-safe"):
            for padding in ("padded", "unpadded"):
                for input_len in lengths:
                    key = (backend, alphabet, padding, input_len)
                    if len(samples[key]) < MINIMUM_SAMPLES:
                        fail(f"missing {MINIMUM_SAMPLES} samples for {key}")
                    values = samples[key]
                    scalar_key = ("scalar", alphabet, padding, input_len)
                    try:
                        validate_advantage(values, samples[scalar_key], minimum_ratio, key)
                    except ValueError as error:
                        fail(str(error))
    for alphabet in ("standard", "url-safe"):
        for padding in ("padded", "unpadded"):
            for input_len in (16 * 1024, 64 * 1024):
                key = ("avx512-vbmi", alphabet, padding, input_len)
                avx2_key = ("avx2", alphabet, padding, input_len)
                if len(samples[key]) < MINIMUM_SAMPLES or len(samples[avx2_key]) < MINIMUM_SAMPLES:
                    fail(f"missing {MINIMUM_SAMPLES} AVX-512/AVX2 samples for {key}")
                if any(
                    not math.isfinite(value) or value <= 0.0
                    for value in samples[key] + samples[avx2_key]
                ):
                    fail(f"invalid observational AVX-512 throughput for {key}")
    print(
        "x86 decode performance: automatic SSSE3/SSE4.1 and AVX2 exceed scalar "
        f"by configured ratio {minimum_ratio:.3f}; AVX-512 remains observational/static"
    )


if __name__ == "__main__":
    main()
