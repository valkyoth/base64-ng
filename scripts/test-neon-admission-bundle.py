#!/usr/bin/env python3
"""Mutation tests for retained native NEON admission bundles."""

from __future__ import annotations

import csv
import hashlib
import subprocess
import tempfile
from pathlib import Path


LENGTHS = (12, 24, 48, 64, 96, 192, 384, 768, 1024, 4096, 64 * 1024)
FILES = ("MANIFEST.txt", "cpu.txt", "neon.csv", "rustc.txt", "uname.txt")
VALIDATOR = Path("scripts/validate-neon-admission-bundle.py").resolve()


def write_csv(path: Path) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            ("backend", "operation", "alphabet", "padding", "input_len",
             "sample_index", "iterations", "elapsed_ns", "throughput_mib_s")
        )
        for backend, throughput in (("scalar", 100.0), ("neon", 110.0)):
            for operation in ("encode", "decode"):
                for alphabet in ("standard", "url-safe"):
                    for padding in ("padded", "unpadded"):
                        for length in LENGTHS:
                            for sample in range(15):
                                writer.writerow(
                                    (backend, operation, alphabet, padding, length,
                                     sample, 1, 1, throughput)
                                )


def write_bundle(path: Path) -> None:
    path.mkdir()
    source = subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
    (path / "MANIFEST.txt").write_text(
        "\n".join(
            (
                "schema=base64-ng-neon-performance-v1",
                f"source_commit={source}",
                "source_status=clean",
                "host=aarch64-unknown-linux-gnu",
                "samples_per_cell=15",
                "target_bytes_per_sample=16777216",
                "median_minimum_ratio=1.02",
                "one_sided_sign_test_maximum_p=0.05",
            )
        )
        + "\n",
        encoding="utf-8",
    )
    (path / "cpu.txt").write_text("fixture CPU\n", encoding="utf-8")
    (path / "rustc.txt").write_text("rustc fixture\n", encoding="utf-8")
    (path / "uname.txt").write_text("Linux fixture\n", encoding="utf-8")
    write_csv(path / "neon.csv")
    checksums = "".join(
        f"{hashlib.sha256((path / name).read_bytes()).hexdigest()}  {name}\n"
        for name in FILES
    )
    (path / "CHECKSUMS.sha256").write_text(checksums, encoding="utf-8")


def run(path: Path, success: bool) -> None:
    result = subprocess.run(
        [str(VALIDATOR), str(path), "--platform", "aarch64-linux"],
        check=False,
        capture_output=True,
        text=True,
    )
    if (result.returncode == 0) != success:
        raise SystemExit(result.stdout + result.stderr)


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        valid = root / "valid"
        write_bundle(valid)
        run(valid, True)

        (valid / "cpu.txt").write_text("tampered\n", encoding="utf-8")
        run(valid, False)

        wrong_host = root / "wrong-host"
        write_bundle(wrong_host)
        manifest = (wrong_host / "MANIFEST.txt").read_text(encoding="utf-8")
        (wrong_host / "MANIFEST.txt").write_text(
            manifest.replace("aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"),
            encoding="utf-8",
        )
        run(wrong_host, False)

    print("NEON admission bundle: mutation checks ok")


if __name__ == "__main__":
    main()
