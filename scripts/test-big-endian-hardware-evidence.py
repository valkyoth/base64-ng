#!/usr/bin/env python3
"""Regression tests for the big-endian evidence validator."""

from __future__ import annotations

import copy
import json
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts/validate-big-endian-hardware-evidence.py"
COMMIT = "1" * 40

VALID = {
    "schema_version": 1,
    "project": "base64-ng",
    "execution_environment": "real-hardware",
    "source_commit": COMMIT,
    "target": {"triple": "s390x-unknown-linux-gnu", "endian": "big"},
    "hardware": {
        "vendor": "example-vendor",
        "model": "example-model",
        "cpu": "example-cpu",
        "features": "example-feature-report",
        "firmware": "example-firmware",
    },
    "software": {
        "os": "example-os",
        "kernel": "example-kernel",
        "rustc": "rustc 1.97.1",
        "cargo": "cargo 1.97.1",
    },
    "verification": {
        "command": "scripts/check_big_endian_hardware.sh",
        "passed": True,
        "output_sha256": "2" * 64,
    },
    "backend": {
        "encode": "scalar",
        "strict_decode": "scalar",
        "secret_decode": "scalar-constant-time-oriented",
        "accelerated": False,
    },
    "review": {
        "reporter": "example-reviewer",
        "recorded_at": "2026-08-02T12:00:00Z",
        "pentest_range": f"{'0' * 40}..{COMMIT}",
        "pentest_result": "PASS",
    },
}


def accepted(report: dict[str, object]) -> bool:
    with tempfile.TemporaryDirectory(prefix="base64-ng-big-endian-evidence-") as directory:
        path = Path(directory) / "report.json"
        path.write_text(json.dumps(report), encoding="utf-8")
        result = subprocess.run(
            [str(VALIDATOR), str(path)],
            cwd=ROOT,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        return result.returncode == 0


assert accepted(VALID)

for mutation in (
    lambda report: report.update(execution_environment="qemu"),
    lambda report: report["backend"].update(accelerated=True),
    lambda report: report["backend"].update(encode="vector"),
    lambda report: report["verification"].update(output_sha256="not-a-hash"),
    lambda report: report["review"].update(pentest_range=f"{'0' * 40}..{'3' * 40}"),
    lambda report: report.update(unreviewed=True),
):
    candidate = copy.deepcopy(VALID)
    mutation(candidate)
    assert not accepted(candidate)

print("big-endian hardware evidence tests: valid report accepted and invalid reports rejected")
