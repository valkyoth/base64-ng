#!/usr/bin/env python3
"""Mutation checks for the Commit 29 NEON performance validator."""

import csv
import subprocess
import tempfile
from pathlib import Path


LENGTHS = (12, 24, 48, 64, 96, 192, 384, 768, 1024, 4096, 64 * 1024)
HEADER = [
    "backend", "operation", "alphabet", "padding", "input_len",
    "sample_index", "iterations", "elapsed_ns", "throughput_mib_s",
]


def rows() -> list[list[object]]:
    evidence = []
    for backend, throughput in (("scalar", 100.0), ("neon", 110.0)):
        for operation in ("encode", "decode"):
            for alphabet in ("standard", "url-safe"):
                for padding in ("padded", "unpadded"):
                    for input_len in LENGTHS:
                        for sample in range(3):
                            evidence.append(
                                [backend, operation, alphabet, padding, input_len,
                                 sample, 1, 1, throughput]
                            )
    return evidence


def write(path: Path, evidence: list[list[object]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(HEADER)
        writer.writerows(evidence)


def validate(path: Path, expect_success: bool) -> None:
    result = subprocess.run(
        ["scripts/validate-neon-performance.py", str(path)],
        check=False,
        capture_output=True,
        text=True,
    )
    if (result.returncode == 0) != expect_success:
        raise SystemExit(result.stdout + result.stderr)


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "evidence.csv"
        evidence = rows()
        write(path, evidence)
        validate(path, True)

        write(path, evidence[:-1])
        validate(path, False)

        slow = rows()
        for row in slow:
            if row[:5] == ["neon", "decode", "url-safe", "unpadded", 192]:
                row[-1] = 99.0
        write(path, slow)
        validate(path, False)
    print("NEON performance validator: mutation checks ok")


if __name__ == "__main__":
    main()
