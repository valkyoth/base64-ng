#!/usr/bin/env python3
"""Capture machine-readable environment metadata for performance evidence."""

from __future__ import annotations

import json
import os
import platform
import subprocess
import sys
from pathlib import Path


def command(*args: str) -> str:
    try:
        return subprocess.run(
            args,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (FileNotFoundError, subprocess.CalledProcessError):
        return "unavailable"


def cpu_fields() -> dict[str, str]:
    path = Path("/proc/cpuinfo")
    if not path.exists():
        return {"model": platform.processor() or "unavailable", "microcode": "unavailable"}
    fields: dict[str, str] = {}
    fallback_processor = ""
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        key, separator, value = line.partition(":")
        if not separator:
            continue
        normalized = key.strip().lower()
        if normalized in {"model name", "hardware"} and "model" not in fields:
            fields["model"] = value.strip()
        if normalized == "processor" and not fallback_processor:
            fallback_processor = value.strip()
        if normalized == "microcode" and "microcode" not in fields:
            fields["microcode"] = value.strip()
    return {
        "model": fields.get(
            "model", platform.processor() or fallback_processor or "unavailable"
        ),
        "microcode": fields.get("microcode", "unavailable"),
    }


def main() -> None:
    output = Path(sys.argv[1])
    data = {
        "schema_version": 1,
        "system": {
            "architecture": platform.machine(),
            "os": platform.platform(),
            "kernel": platform.release(),
            "cpu": cpu_fields(),
        },
        "toolchain": {
            "rustc": command("rustc", "-Vv"),
            "cargo": command("cargo", "-V"),
        },
        "build": {
            "rustflags": os.environ.get("RUSTFLAGS", ""),
            "cargo_profile": "release",
            "target": command("rustc", "-vV").split("host: ")[-1].splitlines()[0],
        },
        "source": {
            "commit": command("git", "rev-parse", "HEAD"),
            "status": command("git", "status", "--short") or "clean",
        },
        "measurement": {
            "campaign_id": os.environ.get("BASE64_NG_PERF_CAMPAIGN_ID", "manual"),
            "sample_count": int(os.environ.get("BASE64_NG_PERF_SAMPLES", "5")),
            "target_bytes_per_sample": int(
                os.environ.get("BASE64_NG_PERF_TARGET_BYTES", str(4 * 1024 * 1024))
            ),
            "cpu_governor": governor(),
        },
    }
    output.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def governor() -> str:
    governors = sorted(
        {
            path.read_text(encoding="utf-8").strip()
            for path in Path("/sys/devices/system/cpu").glob(
                "cpu*/cpufreq/scaling_governor"
            )
        }
    )
    return ",".join(governors) if governors else "unavailable"


if __name__ == "__main__":
    main()
