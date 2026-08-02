#!/usr/bin/env python3
"""Validate Commit 29 exact-backend NEON encode/decode evidence."""

import csv
import math
import os
import statistics
import sys
from collections import defaultdict
from pathlib import Path


LENGTHS = (12, 24, 48, 64, 96, 192, 384, 768, 1024, 4096, 64 * 1024)


def fail(message: str) -> None:
    raise SystemExit(f"NEON performance: {message}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-neon-performance.py <evidence.csv>")
    minimum_ratio = float(os.environ.get("BASE64_NG_NEON_RATIO", "1.02"))
    samples: dict[tuple[str, str, str, str, int], list[float]] = defaultdict(list)
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            key = (
                row["backend"],
                row["operation"],
                row["alphabet"],
                row["padding"],
                int(row["input_len"]),
            )
            samples[key].append(float(row["throughput_mib_s"]))

    for operation in ("encode", "decode"):
        for alphabet in ("standard", "url-safe"):
            for padding in ("padded", "unpadded"):
                for input_len in LENGTHS:
                    neon = ("neon", operation, alphabet, padding, input_len)
                    scalar = ("scalar", operation, alphabet, padding, input_len)
                    if len(samples[neon]) < 3 or len(samples[scalar]) < 3:
                        fail(f"missing three NEON/scalar samples for {neon}")
                    if any(
                        not math.isfinite(value) or value <= 0.0
                        for value in samples[neon] + samples[scalar]
                    ):
                        fail(f"invalid throughput for {neon}")
                    if input_len < 192:
                        continue
                    ratio = statistics.median(samples[neon]) / statistics.median(
                        samples[scalar]
                    )
                    if ratio < minimum_ratio:
                        fail(f"{neon} ratio {ratio:.3f} is below {minimum_ratio:.3f}")
    print(
        "NEON performance: encode and strict decode at automatic sizes exceed scalar "
        f"by configured ratio {minimum_ratio:.3f}"
    )


if __name__ == "__main__":
    main()
