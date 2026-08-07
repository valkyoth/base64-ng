#!/usr/bin/env python3
"""Mutation tests for retained native NEON admission bundles."""

from __future__ import annotations

import csv
import hashlib
import importlib.util
import subprocess
import sys
import tempfile
from pathlib import Path


LENGTHS = (12, 24, 48, 64, 96, 192, 384, 768, 1024, 4096, 64 * 1024)
FILES = ("MANIFEST.txt", "cpu.txt", "neon.csv", "rustc.txt", "uname.txt")
VALIDATOR = Path("scripts/validate-neon-admission-bundle.py").resolve()
SPEC = importlib.util.spec_from_file_location("neon_bundle_validator", VALIDATOR)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR_MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = VALIDATOR_MODULE
SPEC.loader.exec_module(VALIDATOR_MODULE)


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


def write_checksums(path: Path) -> None:
    checksums = "".join(
        f"{hashlib.sha256((path / name).read_bytes()).hexdigest()}  {name}\n"
        for name in FILES
    )
    (path / "CHECKSUMS.sha256").write_text(checksums, encoding="utf-8")


def write_bundle(path: Path) -> None:
    path.mkdir()
    source = subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
    (path / "MANIFEST.txt").write_text(
        "\n".join(
            (
                "schema=base64-ng-neon-performance-v2",
                f"source_commit={source}",
                "source_status=clean",
                "host=aarch64-unknown-linux-gnu",
                "host_metadata_policy=allowlisted-v1",
                "samples_per_cell=15",
                "target_bytes_per_sample=16777216",
                "median_minimum_ratio=1.02",
                "one_sided_sign_test_maximum_p=0.05",
            )
        )
        + "\n",
        encoding="utf-8",
    )
    (path / "cpu.txt").write_text(
        "Architecture: aarch64\n"
        "Byte Order: Little Endian\n"
        "CPU(s): 8\n"
        "Model name: fixture\n"
        "Flags: fp asimd\n",
        encoding="utf-8",
    )
    (path / "rustc.txt").write_text("rustc fixture\n", encoding="utf-8")
    (path / "uname.txt").write_text("Linux 1.0.0 aarch64\n", encoding="utf-8")
    write_csv(path / "neon.csv")
    write_checksums(path)


def run(path: Path, success: bool, *, allow_runtime_drift: bool = False) -> None:
    command = [str(VALIDATOR), str(path), "--platform", "aarch64-linux"]
    if allow_runtime_drift:
        command.append("--allow-runtime-drift")
    result = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
    )
    if (result.returncode == 0) != success:
        raise SystemExit(result.stdout + result.stderr)


def main() -> None:
    source = subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
    assert VALIDATOR_MODULE.is_nonruntime_change("fuzz/fuzz_targets/decode.rs", source)
    assert VALIDATOR_MODULE.is_nonruntime_change("src/v2/formatting_tests.rs", source)
    assert not VALIDATOR_MODULE.is_nonruntime_change("src/v2/formatting.rs", source)
    assert not VALIDATOR_MODULE.is_nonruntime_change("src/v2/mod.rs", source)
    assert VALIDATOR_MODULE.packaging_manifest_equal(
        '[package]\nname = "fixture"\ninclude = ["src/**"]\n',
        '[package]\nname = "fixture"\ninclude = ["src/**", "scripts/*.txt"]\n',
    )
    assert not VALIDATOR_MODULE.packaging_manifest_equal(
        '[package]\nname = "fixture"\n[features]\ndefault = []\n',
        '[package]\nname = "fixture"\n[features]\ndefault = ["std"]\n',
    )

    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        valid = root / "valid"
        write_bundle(valid)
        run(valid, True)
        run(valid, True, allow_runtime_drift=True)

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

        identifying_metadata = root / "identifying-metadata"
        write_bundle(identifying_metadata)
        (identifying_metadata / "uname.txt").write_text(
            "Linux ip-10-0-0-1 1.0.0 aarch64\n", encoding="utf-8"
        )
        write_checksums(identifying_metadata)
        run(identifying_metadata, False)

    print("NEON admission bundle: mutation checks ok")


if __name__ == "__main__":
    main()
