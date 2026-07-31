#!/usr/bin/env python3
"""Opt-in live comparison of the locked RFC 4648 errata index."""

from __future__ import annotations

import csv
import html
import re
import sys
import urllib.request
from pathlib import Path


URL = "https://www.rfc-editor.org/errata/rfc4648"
LOCKED = Path("rfc/rfc4648-errata.tsv")


def main() -> int:
    request = urllib.request.Request(
        URL,
        headers={
            "User-Agent": "base64-ng-rfc-lock/1.0 "
            "(https://github.com/valkyoth/base64-ng)"
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        if response.url.split(":", 1)[0] != "https":
            raise RuntimeError("RFC errata request redirected away from HTTPS")
        document = response.read().decode("utf-8", "strict")

    live: dict[str, tuple[str, str, str]] = {}
    for record in document.split("Errata-ID: ")[1:]:
        identifier_match = re.search(r'href="/eid(\d+)/"', record)
        status_match = re.search(
            r"Status:</dt>.*?<span[^>]*>([^<]+)</span>", record, re.DOTALL
        )
        type_match = re.search(
            r"Type:</dt>.*?<span[^>]*>([^<]+)</span>", record, re.DOTALL
        )
        section_match = re.search(r"Section\s+([^<]+?)\s+says:", record)
        if not all((identifier_match, status_match, type_match, section_match)):
            continue
        identifier = identifier_match.group(1)
        section = html.unescape(section_match.group(1)).strip().rstrip(".")
        live[identifier] = (
            html.unescape(status_match.group(1)).strip(),
            html.unescape(type_match.group(1)).strip(),
            section,
        )

    with LOCKED.open(newline="", encoding="utf-8") as handle:
        locked = {
            row["id"]: (row["status"], row["type"], row["section"].rstrip("."))
            for row in csv.DictReader(handle, delimiter="\t")
        }

    if live != locked:
        print(
            f"RFC errata live check: drift detected; locked={locked!r}, "
            f"live={live!r}",
            file=sys.stderr,
        )
        return 1
    print(f"RFC errata live check: {len(live)} records match")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
