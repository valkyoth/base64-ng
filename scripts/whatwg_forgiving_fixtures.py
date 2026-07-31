#!/usr/bin/env python3
"""Load the locked WHATWG forgiving Base64 browser fixtures."""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
FIXTURE_FILE = ROOT / "tests" / "fixtures" / "whatwg-forgiving-base64.txt"


def load_fixtures() -> list[list[str | None]]:
    fixtures: list[list[str | None]] = []
    for raw_line in FIXTURE_FILE.read_text("utf-8").splitlines():
        if raw_line.startswith("#"):
            continue
        input_hex, expected = raw_line.split("|", 1)
        expected = expected.strip()
        fixtures.append(
            [input_hex.strip(), None if expected == "ERROR" else expected]
        )
    return fixtures


if __name__ == "__main__":
    if sys.argv[1:] == ["--sha256"]:
        print(hashlib.sha256(FIXTURE_FILE.read_bytes()).hexdigest())
    elif not sys.argv[1:]:
        print(json.dumps(load_fixtures(), separators=(",", ":")))
    else:
        raise SystemExit("usage: whatwg_forgiving_fixtures.py [--sha256]")
