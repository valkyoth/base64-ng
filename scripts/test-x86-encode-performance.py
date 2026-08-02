#!/usr/bin/env python3
"""Mutation checks for the Commit 25/26 performance validator."""

import csv
import os
import subprocess
import tempfile
from pathlib import Path


HEADER = [
    "backend",
    "alphabet",
    "padding",
    "input_len",
    "sample_index",
    "iterations",
    "elapsed_ns",
    "throughput_mib_s",
]
LENGTHS = {
    "scalar": (12, 24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
    "ssse3-sse4.1": (12, 24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
    "avx2": (24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
    "avx512-vbmi": (48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
}


def rows() -> list[list[object]]:
    output = []
    for backend, lengths in LENGTHS.items():
        if backend == "scalar":
            throughput = 100.0
        elif backend == "avx512-vbmi":
            throughput = 120.0
        else:
            throughput = 110.0
        for alphabet in ("standard", "url-safe"):
            for padding in ("padded", "unpadded"):
                for input_len in lengths:
                    for sample in range(15):
                        output.append(
                            [backend, alphabet, padding, input_len, sample, 1, 1, throughput]
                        )
    return output


def validate(path: Path, expect_success: bool) -> None:
    result = subprocess.run(
        ["scripts/validate-x86-encode-performance.py", str(path)],
        check=False,
        capture_output=True,
        text=True,
    )
    if (result.returncode == 0) != expect_success:
        raise SystemExit(result.stdout + result.stderr)


def write(path: Path, evidence: list[list[object]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(HEADER)
        writer.writerows(evidence)


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "evidence.csv"
        evidence = rows()
        write(path, evidence)
        validate(path, True)

        missing = evidence[:-1]
        write(path, missing)
        validate(path, False)

        slow = rows()
        for row in slow:
            if row[0] == "avx2" and row[1:4] == ["standard", "padded", 24]:
                row[-1] = 99.0
        write(path, slow)
        validate(path, False)

        slow_avx512 = rows()
        for row in slow_avx512:
            if row[0] == "avx512-vbmi" and row[1:4] == ["standard", "padded", 192]:
                row[-1] = 112.0
        write(path, slow_avx512)
        validate(path, False)

        write(path, rows())
        validate_with_weakened_environment(path)
    print("x86 encode performance validator: mutation checks ok")


def validate_with_weakened_environment(path: Path) -> None:
    environment = os.environ.copy()
    environment["BASE64_NG_X86_ENCODE_RATIO"] = "0.50"
    result = subprocess.run(
        ["scripts/validate-x86-encode-performance.py", str(path)],
        check=False,
        capture_output=True,
        text=True,
        env=environment,
    )
    if result.returncode == 0:
        raise SystemExit("weakened encode threshold was accepted")


if __name__ == "__main__":
    main()
