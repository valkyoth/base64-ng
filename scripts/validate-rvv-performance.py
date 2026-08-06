#!/usr/bin/env python3
"""Validate paired native RVV encode/decode performance evidence."""

import csv
import math
import os
import sys
from collections import defaultdict
from pathlib import Path

from performance_statistics import MINIMUM_SAMPLES, validate_advantage


LENGTHS = (12, 24, 48, 64, 96, 192, 384, 768, 1024, 4096, 64 * 1024)
AUTOMATIC_MINIMUM = 192


def fail(message: str) -> None:
    raise SystemExit(f"RVV performance: {message}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-rvv-performance.py <evidence.csv>")
    minimum_ratio = float(os.environ.get("BASE64_NG_RVV_RATIO", "1.02"))
    if minimum_ratio < 1.02:
        fail("environment cannot weaken the frozen 1.02 admission ratio")
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
                    rvv = ("rvv", operation, alphabet, padding, input_len)
                    scalar = ("scalar", operation, alphabet, padding, input_len)
                    if len(samples[rvv]) < MINIMUM_SAMPLES or len(samples[scalar]) < MINIMUM_SAMPLES:
                        fail(f"missing {MINIMUM_SAMPLES} RVV/scalar samples for {rvv}")
                    if any(
                        not math.isfinite(value) or value <= 0.0
                        for value in samples[rvv] + samples[scalar]
                    ):
                        fail(f"invalid throughput for {rvv}")
                    if input_len < AUTOMATIC_MINIMUM:
                        continue
                    try:
                        validate_advantage(samples[rvv], samples[scalar], minimum_ratio, rvv)
                    except ValueError as error:
                        fail(str(error))
    print(
        "RVV performance: encode and strict decode at proposed automatic sizes "
        f"exceed scalar by configured ratio {minimum_ratio:.3f}"
    )


if __name__ == "__main__":
    main()
