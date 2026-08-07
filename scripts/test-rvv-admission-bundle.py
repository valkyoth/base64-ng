#!/usr/bin/env python3
"""Mutation tests for native RVV admission bundles."""

from __future__ import annotations

import csv
import hashlib
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts/validate-rvv-admission-bundle.py"
FILES = (
    "MANIFEST.txt",
    "asm-attributes.txt",
    "asm-disassembly.txt",
    "asm-manifest.txt",
    "correctness.txt",
    "cpu.txt",
    "rustc.txt",
    "rvv.csv",
    "uname.txt",
)
LENGTHS = (12, 24, 48, 64, 96, 192, 384, 768, 1024, 4096, 64 * 1024)


def write_checksums(directory: Path) -> None:
    (directory / "CHECKSUMS.sha256").write_text(
        "".join(
            f"{hashlib.sha256((directory / name).read_bytes()).hexdigest()}  {name}\n"
            for name in FILES
        ),
        encoding="utf-8",
    )


def write_csv(path: Path) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            ("backend", "operation", "alphabet", "padding", "input_len",
             "sample_index", "iterations", "elapsed_ns", "throughput_mib_s")
        )
        for backend, throughput in (("scalar", 100.0), ("rvv", 110.0)):
            for operation in ("encode", "decode"):
                for alphabet in ("standard", "url-safe"):
                    for padding in ("padded", "unpadded"):
                        for length in LENGTHS:
                            for sample in range(15):
                                writer.writerow(
                                    (backend, operation, alphabet, padding, length,
                                     sample, 1, 1, throughput)
                                )


def write_bundle(directory: Path) -> None:
    directory.mkdir()
    source = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    (directory / "MANIFEST.txt").write_text(
        "\n".join(
            (
                "schema=base64-ng-rvv-native-admission-v2",
                f"source_commit={source}",
                "source_status=clean",
                "host=riscv64gc-unknown-linux-gnu",
                "execution_environment=real-hardware",
                "admission_scope=linux-rvv-1.0-vlen256-spacemit-x60",
                "mvendorid=0x710",
                "marchid=0x8000000058000001",
                "mimpid=0x1000000049772200",
                "vector_length_bits=256",
                "samples_per_cell=15",
                "target_bytes_per_sample=4194304",
                "automatic_encode_minimum_raw_input=384",
                "automatic_decode_minimum_encoded_input=1024",
                "median_minimum_ratio=1.02",
                "one_sided_sign_test_maximum_p=0.05",
                "signal_context=pass",
                "thread_context=pass",
                "ffi_abi=pass",
                "register_cleanup=pass",
            )
        )
        + "\n",
        encoding="utf-8",
    )
    (directory / "asm-attributes.txt").write_text("Tag_RISCV_arch: rv64gcv\n")
    (directory / "asm-disassembly.txt").write_text(
        "base64_ng_rvv_encode_standard_quanta\n"
        "base64_ng_rvv_decode_standard_quanta\n"
        "base64_ng_rvv_signal_context_round_trip\n"
        "base64_ng_rvv_signal_clobber\n"
        "amoswap.w\n"
        "vmv.v.i v15,0\n"
    )
    (directory / "asm-manifest.txt").write_text("production_admission=false\n")
    (directory / "correctness.txt").write_text(
        "RVV candidate VLEN=256 bits\n"
        "rvv_state_survives_linux_signal_delivery ... ok\n"
        "rvv_candidate_survives_thread_context_switches ... ok\n"
        "RISC-V hardware checks: ok\n"
    )
    (directory / "cpu.txt").write_text(
        "Architecture: riscv64\nModel name: Spacemit(R) X60\nFlags: v\n"
        "mvendorid: 0x710\n"
        "marchid: 0x8000000058000001\n"
        "mimpid: 0x1000000049772200\n"
    )
    (directory / "rustc.txt").write_text("rustc fixture\n")
    (directory / "uname.txt").write_text("Linux 1.0 riscv64\n")
    write_csv(directory / "rvv.csv")
    write_checksums(directory)


def run(directory: Path, success: bool) -> None:
    result = subprocess.run(
        [str(VALIDATOR), str(directory)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if (result.returncode == 0) != success:
        raise AssertionError(result.stdout + result.stderr)


def set_rvv_throughput(
    path: Path, operation: str, input_len: int, throughput: float
) -> None:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle))
    for row in rows[1:]:
        if row[0] == "rvv" and row[1] == operation and int(row[4]) == input_len:
            row[8] = str(throughput)
    with path.open("w", newline="", encoding="utf-8") as handle:
        csv.writer(handle).writerows(rows)


def main() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        valid = root / "valid"
        write_bundle(valid)
        run(valid, True)

        tampered = root / "tampered"
        shutil.copytree(valid, tampered)
        (tampered / "correctness.txt").write_text("forged pass\n")
        run(tampered, False)

        weak = root / "weak"
        shutil.copytree(valid, weak)
        manifest = (weak / "MANIFEST.txt").read_text()
        (weak / "MANIFEST.txt").write_text(
            manifest.replace("samples_per_cell=15", "samples_per_cell=1")
        )
        write_checksums(weak)
        run(weak, False)

        below_threshold = root / "below-threshold"
        shutil.copytree(valid, below_threshold)
        set_rvv_throughput(below_threshold / "rvv.csv", "encode", 192, 50.0)
        set_rvv_throughput(below_threshold / "rvv.csv", "decode", 384, 50.0)
        write_checksums(below_threshold)
        run(below_threshold, True)

        slow_encode = root / "slow-encode"
        shutil.copytree(valid, slow_encode)
        set_rvv_throughput(slow_encode / "rvv.csv", "encode", 384, 100.0)
        write_checksums(slow_encode)
        run(slow_encode, False)

        slow_decode = root / "slow-decode"
        shutil.copytree(valid, slow_decode)
        set_rvv_throughput(slow_decode / "rvv.csv", "decode", 768, 100.0)
        write_checksums(slow_decode)
        run(slow_decode, False)

        for name in (*FILES, "CHECKSUMS.sha256"):
            linked = root / f"linked-{name}"
            shutil.copytree(valid, linked)
            artifact = linked / name
            artifact.unlink()
            artifact.symlink_to(valid / name)
            run(linked, False)

        linked_root = root / "linked-root"
        linked_root.symlink_to(valid, target_is_directory=True)
        run(linked_root, False)
    print("RVV admission bundle: mutation checks ok")


if __name__ == "__main__":
    main()
