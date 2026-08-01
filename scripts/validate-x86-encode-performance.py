#!/usr/bin/env python3
"""Validate Commit 25/26 exact-backend encode performance evidence."""

import csv
import math
import os
import statistics
import sys
from collections import defaultdict
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(f"x86 encode performance: {message}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-x86-encode-performance.py <evidence.csv>")
    path = Path(sys.argv[1])
    minimum_ratio = float(os.environ.get("BASE64_NG_X86_ENCODE_RATIO", "1.02"))
    avx512_to_avx2_ratio = float(
        os.environ.get("BASE64_NG_AVX512_TO_AVX2_RATIO", "1.05")
    )
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
        "avx512-vbmi": (48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
    }
    for backend, lengths in required_lengths.items():
        for alphabet in ("standard", "url-safe"):
            for padding in ("padded", "unpadded"):
                for input_len in lengths:
                    key = (backend, alphabet, padding, input_len)
                    if len(samples[key]) < 3:
                        fail(f"missing three samples for {key}")
    for key, values in sorted(samples.items()):
        backend, alphabet, padding, input_len = key
        if backend == "scalar":
            continue
        if any(not math.isfinite(value) or value <= 0.0 for value in values):
            fail(f"invalid throughput for {key}")
        if backend == "avx512-vbmi" and input_len < 192:
            continue
        scalar_key = ("scalar", alphabet, padding, input_len)
        if len(samples[scalar_key]) < 3:
            fail(f"missing three scalar samples for {key}")
        ratio = statistics.median(values) / statistics.median(samples[scalar_key])
        if ratio < minimum_ratio:
            fail(f"{key} ratio {ratio:.3f} is below {minimum_ratio:.3f}")
        if backend == "avx512-vbmi":
            avx2_key = ("avx2", alphabet, padding, input_len)
            if len(samples[avx2_key]) < 3:
                fail(f"missing three AVX2 comparison samples for {key}")
            width_ratio = statistics.median(values) / statistics.median(samples[avx2_key])
            if width_ratio < avx512_to_avx2_ratio:
                fail(
                    f"{key} AVX-512/AVX2 ratio {width_ratio:.3f} "
                    f"is below {avx512_to_avx2_ratio:.3f}"
                )
    print(
        "x86 encode performance: SSSE3/SSE4.1, AVX2, and AVX-512 admitted sizes exceed scalar by "
        f"configured scalar ratio {minimum_ratio:.3f}; automatic AVX-512 sizes exceed "
        f"AVX2 by {avx512_to_avx2_ratio:.3f}"
    )


if __name__ == "__main__":
    main()
