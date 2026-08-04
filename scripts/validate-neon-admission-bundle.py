#!/usr/bin/env python3
"""Validate a retained native NEON admission bundle."""

from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import sys
from pathlib import Path


FILES = {
    "CHECKSUMS.sha256",
    "MANIFEST.txt",
    "cpu.txt",
    "neon.csv",
    "rustc.txt",
    "uname.txt",
}
CHECKSUMMED = FILES - {"CHECKSUMS.sha256"}
KEYS = {
    "schema",
    "source_commit",
    "source_status",
    "host",
    "host_metadata_policy",
    "samples_per_cell",
    "target_bytes_per_sample",
    "median_minimum_ratio",
    "one_sided_sign_test_maximum_p",
}
APPLE_CPU_KEYS = {
    "hw.machine",
    "hw.model",
    "hw.ncpu",
    "hw.physicalcpu",
    "hw.logicalcpu",
    "hw.memsize",
    "hw.byteorder",
    "hw.optional.neon",
    "hw.optional.arm64",
    "machdep.cpu.brand_string",
}
APPLE_CPU_REQUIRED = {
    "hw.ncpu",
    "hw.byteorder",
    "hw.optional.neon",
    "hw.optional.arm64",
    "machdep.cpu.brand_string",
}
LINUX_CPU_KEYS = {
    "Architecture",
    "CPU op-mode(s)",
    "Byte Order",
    "CPU(s)",
    "On-line CPU(s) list",
    "Vendor ID",
    "Model name",
    "Model",
    "Thread(s) per core",
    "Core(s) per socket",
    "Socket(s)",
    "Stepping",
    "BogoMIPS",
    "Flags",
    "L1d cache",
    "L1i cache",
    "L2 cache",
    "L3 cache",
    "Virtualization",
    "Hypervisor vendor",
}
LINUX_CPU_REQUIRED = {"Architecture", "Byte Order", "CPU(s)", "Model name", "Flags"}
RUNTIME_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "src",
    "crates",
    "packages",
    "fuzz",
    "perf",
    "portability",
)


def fail(message: str) -> None:
    raise SystemExit(f"NEON admission bundle: {message}")


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


def parse_checksums(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    pattern = re.compile(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)")
    for line in path.read_text(encoding="utf-8").splitlines():
        match = pattern.fullmatch(line)
        if match is None:
            fail(f"malformed checksum line: {line}")
        digest, name = match.groups()
        if name in result:
            fail(f"duplicate checksum entry: {name}")
        result[name] = digest
    if set(result) != CHECKSUMMED:
        fail("checksum inventory does not match the frozen bundle schema")
    return result


def check_git_source(source: str) -> None:
    if re.fullmatch(r"[0-9a-f]{40}", source) is None:
        fail("source_commit must be one exact 40-hex commit")
    if subprocess.run(
        ["git", "cat-file", "-e", f"{source}^{{commit}}"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode != 0:
        fail(f"source commit is unavailable: {source}")
    if subprocess.run(
        ["git", "merge-base", "--is-ancestor", source, "HEAD"], check=False
    ).returncode != 0:
        fail("source commit is not an ancestor of HEAD")

    changed = subprocess.run(
        ["git", "diff", "--name-only", f"{source}..HEAD", "--", *RUNTIME_PATHS],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if changed:
        fail(f"runtime source changed after capture:\n{changed}")


def validate_host_metadata(directory: Path, host: str) -> None:
    cpu = (directory / "cpu.txt").read_text(encoding="utf-8")
    uname = (directory / "uname.txt").read_text(encoding="utf-8").strip()
    lines = cpu.splitlines()
    if any(":" not in line for line in lines):
        fail("CPU metadata contains a malformed line")
    keys = {line.split(":", 1)[0] for line in lines}

    if host == "aarch64-apple-darwin":
        if not APPLE_CPU_REQUIRED <= keys or not keys <= APPLE_CPU_KEYS:
            fail("Apple CPU metadata does not match the allowlist")
        if re.fullmatch(r"Darwin [0-9][A-Za-z0-9._-]* arm64", uname) is None:
            fail("Apple OS metadata is not minimized")
    else:
        if not LINUX_CPU_REQUIRED <= keys or not keys <= LINUX_CPU_KEYS:
            fail("Linux CPU metadata does not match the allowlist")
        if re.fullmatch(r"Linux [0-9][A-Za-z0-9._+-]* aarch64", uname) is None:
            fail("Linux OS metadata is not minimized")

    combined = f"{cpu}\n{uname}"
    forbidden = re.compile(
        r"(?i)(hostname|bootuuid|bootsessionuuid|machine.?id|/users/|/home/|"
        r"ip-[0-9]+-[0-9]+-[0-9]+-[0-9]+|"
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})"
    )
    if forbidden.search(combined):
        fail("host metadata contains a machine-specific identifier")


def validate(directory: Path, platform: str | None) -> None:
    if not directory.is_dir():
        fail(f"bundle directory is missing: {directory}")
    actual = {entry.name for entry in directory.iterdir() if entry.is_file()}
    if actual != FILES:
        fail("bundle file inventory does not match the frozen schema")
    for name in FILES:
        if (directory / name).stat().st_size == 0:
            fail(f"bundle artifact is empty: {name}")

    manifest = parse_manifest(directory / "MANIFEST.txt")
    if manifest["schema"] != "base64-ng-neon-performance-v2":
        fail("unsupported schema")
    if manifest["source_status"] != "clean":
        fail("capture source was not clean")
    if manifest["host_metadata_policy"] != "allowlisted-v1":
        fail("host metadata policy does not match the frozen allowlist")
    if int(manifest["samples_per_cell"]) < 15:
        fail("fewer than 15 samples per cell")
    if int(manifest["target_bytes_per_sample"]) <= 0:
        fail("target bytes per sample must be positive")
    if manifest["median_minimum_ratio"] != "1.02":
        fail("median ratio does not match the frozen 1.02 policy")
    if manifest["one_sided_sign_test_maximum_p"] != "0.05":
        fail("sign-test threshold does not match the frozen 0.05 policy")

    host = manifest["host"]
    expected = {
        "apple-silicon": "aarch64-apple-darwin",
        "aarch64-linux": "aarch64-unknown-linux-gnu",
    }
    if platform is not None and host != expected[platform]:
        fail(f"{platform} bundle has unexpected host: {host}")
    if host not in expected.values():
        fail(f"host is not an admitted native NEON evidence host: {host}")
    validate_host_metadata(directory, host)

    checksums = parse_checksums(directory / "CHECKSUMS.sha256")
    for name, expected_digest in checksums.items():
        actual_digest = hashlib.sha256((directory / name).read_bytes()).hexdigest()
        if actual_digest != expected_digest:
            fail(f"checksum mismatch: {name}")

    check_git_source(manifest["source_commit"])
    subprocess.run(
        [sys.executable, "scripts/validate-neon-performance.py", str(directory / "neon.csv")],
        check=True,
    )
    print(f"NEON admission bundle: {platform or host} ok")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--platform", choices=("apple-silicon", "aarch64-linux"))
    args = parser.parse_args()
    validate(args.directory, args.platform)


if __name__ == "__main__":
    main()
