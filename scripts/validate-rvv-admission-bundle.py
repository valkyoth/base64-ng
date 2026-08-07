#!/usr/bin/env python3
"""Validate a retained native RISC-V Vector admission bundle."""

from __future__ import annotations

import hashlib
import re
import subprocess
import sys
from pathlib import Path


FILES = {
    "CHECKSUMS.sha256",
    "MANIFEST.txt",
    "asm-attributes.txt",
    "asm-disassembly.txt",
    "asm-manifest.txt",
    "correctness.txt",
    "cpu.txt",
    "rustc.txt",
    "rvv.csv",
    "uname.txt",
}
CHECKSUMMED = FILES - {"CHECKSUMS.sha256"}
KEYS = {
    "schema",
    "source_commit",
    "source_status",
    "host",
    "execution_environment",
    "admission_scope",
    "mvendorid",
    "marchid",
    "mimpid",
    "vector_length_bits",
    "samples_per_cell",
    "target_bytes_per_sample",
    "automatic_minimum_input",
    "median_minimum_ratio",
    "one_sided_sign_test_maximum_p",
    "signal_context",
    "thread_context",
    "ffi_abi",
    "register_cleanup",
}


def fail(message: str) -> None:
    raise SystemExit(f"RVV admission bundle: {message}")


def parse_manifest(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in line:
            fail(f"malformed manifest line: {line}")
        key, value = line.split("=", 1)
        if key in result:
            fail(f"duplicate manifest key: {key}")
        result[key] = value
    if set(result) != KEYS:
        fail("manifest keys do not match the frozen schema")
    return result


def validate(directory: Path) -> None:
    if directory.is_symlink() or not directory.is_dir():
        fail(f"bundle directory is missing: {directory}")
    entries = list(directory.iterdir())
    if any(entry.is_symlink() or not entry.is_file() for entry in entries):
        fail("bundle must contain only regular, non-symlink files")
    actual = {entry.name for entry in entries}
    if actual != FILES:
        fail("bundle file inventory does not match the frozen schema")
    for name in FILES:
        artifact = directory / name
        if artifact.is_symlink() or not artifact.is_file():
            fail(f"bundle artifact is not a regular file: {name}")
        if artifact.stat().st_size == 0:
            fail(f"bundle contains an empty artifact: {name}")

    manifest = parse_manifest(directory / "MANIFEST.txt")
    expected = {
        "schema": "base64-ng-rvv-native-admission-v1",
        "source_status": "clean",
        "execution_environment": "real-hardware",
        "admission_scope": "linux-rvv-1.0-vlen256-spacemit-x60",
        "mvendorid": "0x710",
        "marchid": "0x8000000058000001",
        "mimpid": "0x1000000049772200",
        "vector_length_bits": "256",
        "automatic_minimum_input": "192",
        "median_minimum_ratio": "1.02",
        "one_sided_sign_test_maximum_p": "0.05",
        "signal_context": "pass",
        "thread_context": "pass",
        "ffi_abi": "pass",
        "register_cleanup": "pass",
    }
    for key, value in expected.items():
        if manifest[key] != value:
            fail(f"unexpected {key}: {manifest[key]}")
    if not manifest["host"].startswith("riscv64") or not manifest["host"].endswith("linux-gnu"):
        fail("host is not native riscv64 Linux GNU")
    if int(manifest["samples_per_cell"]) < 15 or int(manifest["target_bytes_per_sample"]) <= 0:
        fail("performance sample policy is too weak")
    source = manifest["source_commit"]
    if re.fullmatch(r"[0-9a-f]{40}", source) is None:
        fail("source_commit is not exact")
    if subprocess.run(
        ["git", "cat-file", "-e", f"{source}^{{commit}}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    ).returncode != 0:
        fail(f"source commit is unavailable: {source}")

    checksums: dict[str, str] = {}
    pattern = re.compile(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)")
    for line in (directory / "CHECKSUMS.sha256").read_text(encoding="utf-8").splitlines():
        match = pattern.fullmatch(line)
        if match is None or match.group(2) in checksums:
            fail("malformed or duplicate checksum entry")
        checksums[match.group(2)] = match.group(1)
    if set(checksums) != CHECKSUMMED:
        fail("checksum inventory does not match the bundle schema")
    for name, expected_digest in checksums.items():
        if hashlib.sha256((directory / name).read_bytes()).hexdigest() != expected_digest:
            fail(f"checksum mismatch: {name}")

    cpu = (directory / "cpu.txt").read_text(encoding="utf-8")
    correctness = (directory / "correctness.txt").read_text(encoding="utf-8")
    assembly = (directory / "asm-disassembly.txt").read_text(encoding="utf-8")
    if "Architecture:" not in cpu or "riscv64" not in cpu or "Spacemit(R) X60" not in cpu:
        fail("CPU metadata does not identify the reviewed X60 RISC-V host")
    for marker in (
        "mvendorid: 0x710",
        "marchid: 0x8000000058000001",
        "mimpid: 0x1000000049772200",
    ):
        if marker not in cpu:
            fail(f"CPU metadata is missing {marker}")
    for marker in (
        "RVV candidate VLEN=256 bits",
        "rvv_state_survives_linux_signal_delivery",
        "rvv_candidate_survives_thread_context_switches",
        "RISC-V hardware checks: ok",
    ):
        if marker not in correctness:
            fail(f"correctness transcript is missing {marker}")
    for marker in (
        "base64_ng_rvv_encode_standard_quanta",
        "base64_ng_rvv_decode_standard_quanta",
        "base64_ng_rvv_signal_context_round_trip",
        "base64_ng_rvv_signal_clobber",
        "vmv.v.i",
        "amoswap.w",
    ):
        if marker not in assembly:
            fail(f"assembly evidence is missing {marker}")

    subprocess.run(
        [sys.executable, "scripts/validate-rvv-performance.py", str(directory / "rvv.csv")],
        check=True,
    )
    print("RVV admission bundle: native X60 Linux evidence ok")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-rvv-admission-bundle.py <bundle-directory>")
    validate(Path(sys.argv[1]))


if __name__ == "__main__":
    main()
