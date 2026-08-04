#!/usr/bin/env python3
"""Validate the authoritative 2.0 checkpoint table."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


HASH = re.compile(r"[0-9a-f]{40}")
FINAL_COMMIT_55 = "Report-only release commit (HEAD)"
FINAL_EVIDENCE_55 = "[v2.0.0 report](security/pentest/v2.0.0.md)"


def fail(message: str) -> None:
    raise SystemExit(f"2.0 checkpoint record: {message}")


def table_rows(path: Path) -> list[list[str]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    try:
        start = lines.index("## Checkpoint Record")
    except ValueError:
        fail(f"{path} has no Checkpoint Record section")

    rows: list[list[str]] = []
    for line in lines[start + 1 :]:
        if line.startswith("## "):
            break
        if not re.match(r"^\| [0-9]+ \|", line):
            continue
        columns = [column.strip() for column in line.strip().strip("|").split("|")]
        if len(columns) != 5:
            fail(f"malformed checkpoint row: {line}")
        rows.append(columns)
    return rows


def validate(path: Path, final: bool) -> None:
    rows = table_rows(path)
    numbers = [int(row[0]) for row in rows]
    if numbers != list(range(1, 56)):
        fail("checkpoint rows must contain each commit from 1 through 55 exactly once")

    for number, subject, accepted, pentest, evidence in rows:
        if not subject or not accepted or not pentest or not evidence:
            fail(f"commit {number} contains an empty cell")

    if not final:
        print("2.0 checkpoint record: development table shape ok")
        return

    for number, _subject, accepted, pentest, evidence in rows:
        cells = (accepted, pentest, evidence)
        if any("pending" in cell.casefold() for cell in cells):
            fail(f"commit {number} remains pending")
        if pentest != "PASS":
            fail(f"commit {number} pentest disposition must be exactly PASS")

        commit = int(number)
        if commit <= 54:
            normalized = accepted.strip("`")
            if HASH.fullmatch(normalized) is None:
                fail(f"commit {number} accepted hash must be one exact 40-hex commit")
        elif accepted != FINAL_COMMIT_55:
            fail(f"commit 55 accepted value must be: {FINAL_COMMIT_55}")
        elif evidence != FINAL_EVIDENCE_55:
            fail(f"commit 55 evidence must be: {FINAL_EVIDENCE_55}")

    print("2.0 checkpoint record: final table complete")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--final", action="store_true")
    parser.add_argument("plan", nargs="?", default="2.0.0-release-plan.md")
    args = parser.parse_args()
    validate(Path(args.plan), args.final)


if __name__ == "__main__":
    main()
