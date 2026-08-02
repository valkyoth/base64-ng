#!/usr/bin/env python3
"""Adversarial regression tests for the real-SVE evidence validator."""

from __future__ import annotations

import copy
import json
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts/validate-sve-hardware-evidence.py"
COMMIT = "1" * 40
VALID = {
    "schema_version": 1, "project": "base64-ng", "execution_environment": "real-hardware", "source_commit": COMMIT,
    "target": {"triple": "aarch64-unknown-linux-gnu", "arch": "aarch64", "endian": "little"},
    "hardware": {"vendor": "vendor", "system": "system", "soc": "soc", "cpu": "cpu", "firmware": "firmware"},
    "software": {"os": "os", "kernel": "kernel", "rustc": "rustc 1.97.1", "cargo": "cargo 1.97.1"},
    "vector": {"specification": "SVE", "vector_length_bits": 256, "hwcap_sve": True, "prctl_vector_length": True, "per_thread_vl_review": "PASS", "signal_context_review": "PASS", "ffi_abi_review": "PASS"},
    "verification": {"command": "scripts/check_sve_hardware.sh", "passed": True, "output_sha256": "2" * 64},
    "backend": {"encode": "sve-candidate", "strict_decode": "sve-candidate", "secret_decode": "scalar-constant-time-oriented", "production_admitted": False},
    "benchmark": {"command": "reviewed benchmark command", "encode_beneficial": True, "decode_beneficial": True, "raw_data_sha256": "3" * 64},
    "review": {"reporter": "reviewer", "recorded_at": "2026-08-02T12:00:00Z", "assembly": "PASS", "register_cleanup": "PASS", "pentest_range": f"{'0' * 40}..{COMMIT}", "pentest_result": "PASS"},
}


def accepted(report: dict[str, object]) -> bool:
    with tempfile.TemporaryDirectory(prefix="base64-ng-sve-evidence-") as directory:
        path = Path(directory) / "report.json"
        path.write_text(json.dumps(report), encoding="utf-8")
        result = subprocess.run([str(VALIDATOR), str(path)], cwd=ROOT, check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        return result.returncode == 0


assert accepted(VALID)
for mutation in (
    lambda report: report.update(execution_environment="qemu"),
    lambda report: report["vector"].update(vector_length_bits=64),
    lambda report: report["vector"].update(hwcap_sve=False),
    lambda report: report["vector"].update(prctl_vector_length=False),
    lambda report: report["backend"].update(production_admitted=True),
    lambda report: report["benchmark"].update(decode_beneficial=False),
    lambda report: report["review"].update(pentest_range=f"{'0' * 40}..{'4' * 40}"),
    lambda report: report.update(unreviewed=True),
):
    candidate = copy.deepcopy(VALID)
    mutation(candidate)
    assert not accepted(candidate)

print("SVE hardware evidence tests: valid report accepted and invalid reports rejected")
