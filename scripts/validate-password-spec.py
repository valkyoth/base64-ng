#!/usr/bin/env python3
"""Validate the retained password-format sources and requirement mapping."""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = ROOT / "spec/password"
EXPECTED = {
    "passlib-1.7.4-pbkdf2.rst": "924c8d9fecc04fbe99dae4a88771617b8ec8590a7ad918540d00d04e8116dd11",
    "SHA-crypt.txt": "e06bddaf32416914e3b3bd5a155102ab764455a322ea15f5e9eec1fa114b73a8",
}


def fail(message: str) -> None:
    raise SystemExit(f"password spec: {message}")


for name, expected in EXPECTED.items():
    path = SPEC / name
    if not path.is_file():
        fail(f"missing {name}")
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        fail(f"checksum mismatch for {name}")

passlib = (SPEC / "passlib-1.7.4-pbkdf2.rst").read_text(encoding="utf-8")
for required in [
    "$pbkdf2-{digest}${rounds}${salt}${checksum}",
    "pbkdf2_sha256",
    "ab64_encode",
]:
    if required not in passlib:
        fail(f"Passlib source is missing {required!r}")

sha_crypt = (SPEC / "SHA-crypt.txt").read_text(encoding="utf-8")
for required in ["$5$", "$6$", "rounds=", "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"]:
    if required not in sha_crypt:
        fail(f"SHA-crypt source is missing {required!r}")

requirements = json.loads((SPEC / "requirements.json").read_text(encoding="utf-8"))
ids = {row["id"] for row in requirements.get("requirements", [])}
if ids != {"PASSLIB-PBKDF2-FORMAT", "SHA-CRYPT-FORMAT"}:
    fail("requirement identifiers changed")

print("password spec: exact Passlib 1.7.4 and SHA-crypt sources ok")
