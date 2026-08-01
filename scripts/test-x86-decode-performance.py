#!/usr/bin/env python3
"""Mutation checks for the Commit 27 strict-decode performance validator."""

import csv
import subprocess
import tempfile
from pathlib import Path


HEADER = [
    "backend", "alphabet", "padding", "input_len", "sample_index",
    "iterations", "elapsed_ns", "throughput_mib_s",
]
LENGTHS = {
    "scalar": (12, 24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
    "ssse3-sse4.1": (12, 24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
    "avx2": (24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024),
}


def rows() -> list[list[object]]:
    output = []
    for backend, lengths in LENGTHS.items():
        throughput = 100.0 if backend == "scalar" else 110.0
        for alphabet in ("standard", "url-safe"):
            for padding in ("padded", "unpadded"):
                for input_len in lengths:
                    for sample in range(3):
                        output.append(
                            [backend, alphabet, padding, input_len, sample, 1, 1, throughput]
                        )
    return output


def write(path: Path, evidence: list[list[object]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(HEADER)
        writer.writerows(evidence)


def validate(path: Path, expect_success: bool) -> None:
    result = subprocess.run(
        ["scripts/validate-x86-decode-performance.py", str(path)],
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
            if row[0] == "avx2" and row[1:4] == ["standard", "padded", 24]:
                row[-1] = 99.0
        write(path, slow)
        validate(path, False)
    print("x86 decode performance validator: mutation checks ok")


if __name__ == "__main__":
    main()
