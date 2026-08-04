#!/usr/bin/env python3
"""Mutation tests for the 2.0 checkpoint release boundary."""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path


VALIDATOR = Path("scripts/validate-2.0-checkpoint-record.py").resolve()


def write_plan(path: Path, mutation: str | None = None) -> None:
    lines = ["# Fixture", "", "## Checkpoint Record", ""]
    for number in range(1, 56):
        accepted = f"`{number:040x}`"
        evidence = f"evidence-{number}"
        if number == 55:
            accepted = "Report-only release commit (HEAD)"
            evidence = "[v2.0.0 report](security/pentest/v2.0.0.md)"
        pentest = "PASS"
        if mutation == f"pending-{number}":
            evidence = "Pending"
        if mutation == f"hash-{number}":
            accepted = "deadbeef"
        if mutation == f"pentest-{number}":
            pentest = "PASS with caveat"
        lines.append(
            f"| {number} | Subject {number} | {accepted} | {pentest} | {evidence} |"
        )
    if mutation == "missing-54":
        lines = [line for line in lines if not line.startswith("| 54 |")]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def run(path: Path, *arguments: str, success: bool) -> None:
    result = subprocess.run(
        [str(VALIDATOR), *arguments, str(path)],
        check=False,
        capture_output=True,
        text=True,
    )
    if (result.returncode == 0) != success:
        raise SystemExit(result.stdout + result.stderr)


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        plan = Path(directory) / "plan.md"
        write_plan(plan)
        run(plan, success=True)
        run(plan, "--final", success=True)

        for mutation in (
            "pending-1",
            "pending-55",
            "hash-1",
            "pentest-54",
            "missing-54",
        ):
            write_plan(plan, mutation)
            run(plan, "--final", success=False)

    print("2.0 checkpoint record: mutation checks ok")


if __name__ == "__main__":
    main()
