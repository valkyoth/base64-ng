#!/usr/bin/env python3
"""Validate the frozen 2.0 protocol and interoperability registry offline."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import pathlib
import re
import sys

PROTOCOL_HEADER = [
    "id", "package", "public_scope", "specification", "publication", "source",
    "errata", "requirements", "corpus_prefix", "limits",
    "independent_implementations", "api_snapshot",
]
CONFIG_HEADER = [
    "id", "public_symbol", "family", "source", "errata", "requirements",
    "corpus_case", "reference", "version", "checksum",
]
CASE_HEADER = [
    "id", "registry_id", "decision", "plain_hex", "wire_hex", "source_sha256",
    "errata_sha256", "requirement_ids", "provenance_ids",
]
PROVENANCE_HEADER = [
    "id", "implementation", "version_or_snapshot", "license", "role", "reproduction",
]
PROTOCOL_IDS = {
    "mime-body", "pem-textual", "multibase-base64", "imap-mutf7-payload",
    "passlib-pbkdf2", "sha-crypt", "openpgp-armor",
}
REFERENCE_LOCKS = {
    ("base64", "0.23.0"): "b25655df2c3cdd83c5e5b293b88acd880332b2ddadd7c30ac43144fdc0033da9",
    ("base64ct", "1.8.3"): "2af50177e190e07a26ab74f8b1efbfe2ef87da2116221318cb1c2e82baf7de06",
}
EVIDENCE_LICENSES = {
    "Apache-2.0", "BSD-3-Clause", "GPL-3.0-or-later", "LGPL-2.0-or-later",
    "LGPL-2.1-or-later", "LicenseRef-Public-Domain", "MIT;Apache-2.0",
    "MIT;CC-BY-SA-4.0", "PSF-2.0",
}


def fail(message: str) -> None:
    raise SystemExit(f"protocol registry: {message}")


def rows(
    path: pathlib.Path,
    header: list[str],
    *,
    allow_empty: set[str] | None = None,
) -> list[dict[str, str]]:
    allow_empty = allow_empty or set()
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        if reader.fieldnames != header:
            fail(f"unexpected header in {path}")
        result = list(reader)
    if not result or any(
        None in row.values()
        or any(value == "" and key not in allow_empty for key, value in row.items())
        for row in result
    ):
        fail(f"empty or incomplete row in {path}")
    ids = [row["id"] for row in result]
    if len(ids) != len(set(ids)):
        fail(f"duplicate identifiers in {path}")
    return result


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def requirement_ids(path: pathlib.Path) -> set[str]:
    document = json.loads(path.read_text(encoding="utf-8"))
    result = {entry["id"] for entry in document.get("requirements", [])}
    if not result:
        fail(f"empty requirement ledger {path}")
    return result


def public_configuration_names(root: pathlib.Path) -> set[str]:
    result: set[str] = set()
    for relative in ["src/v2/specifications.rs", "src/v2/profiles.rs"]:
        text = (root / relative).read_text(encoding="utf-8")
        result.update(re.findall(r"^pub const ([A-Z][A-Z0-9_]+):", text, re.MULTILINE))
    compat = (root / "src/v2/compat.rs").read_text(encoding="utf-8")
    result.update(re.findall(r"preset!\(\s*([A-Z][A-Z0-9_]+),", compat))
    alphabet = (root / "src/v2/alphabet.rs").read_text(encoding="utf-8")
    if re.search(r"^pub const BINHEX_ALPHABET:", alphabet, re.MULTILINE):
        result.add("BINHEX_ALPHABET")
    return result


def validate_hash_inventory(root: pathlib.Path, required: set[str]) -> None:
    inventory = root / "protocol-registry/v1/SHA256SUMS"
    recorded: dict[str, str] = {}
    for line in inventory.read_text(encoding="utf-8").splitlines():
        digest, relative = line.split("  ", 1)
        if relative in recorded:
            fail(f"duplicate hash inventory path {relative}")
        recorded[relative] = digest
    if set(recorded) != required:
        missing = sorted(required - set(recorded))
        extra = sorted(set(recorded) - required)
        fail(f"hash inventory scope changed; missing={missing}, extra={extra}")
    for relative, expected in recorded.items():
        path = root / relative
        if not path.is_file() or sha256(path) != expected:
            fail(f"hash mismatch for {relative}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    args = parser.parse_args()
    root = args.root.resolve()
    registry = root / "protocol-registry/v1"

    protocols = rows(registry / "protocols.tsv", PROTOCOL_HEADER)
    configs = rows(registry / "configurations.tsv", CONFIG_HEADER)
    cases = rows(
        registry / "cases.tsv",
        CASE_HEADER,
        allow_empty={"plain_hex", "wire_hex"},
    )
    provenance = rows(registry / "provenance.tsv", PROVENANCE_HEADER)
    if {row["id"] for row in protocols} != PROTOCOL_IDS:
        fail("protocol identifier set changed")

    case_by_id = {row["id"]: row for row in cases}
    provenance_ids = {row["id"] for row in provenance}
    for item in provenance:
        if item["license"] not in EVIDENCE_LICENSES:
            fail(f"unreviewed evidence license for {item['id']}: {item['license']}")
        for relative in item["reproduction"].split("|"):
            if not (root / relative).exists():
                fail(f"missing reproduction evidence for {item['id']}: {relative}")
    required_hashes = {
        "protocol-registry/v1/schema.json", "protocol-registry/v1/protocols.tsv",
        "protocol-registry/v1/configurations.tsv", "protocol-registry/v1/cases.tsv",
        "protocol-registry/v1/provenance.tsv", "protocol-registry/runner/Cargo.toml",
        "protocol-registry/runner/Cargo.lock", "protocol-registry/runner/deny.toml",
        "protocol-registry/runner/src/main.rs", "protocol-registry/runner/src/model.rs",
        "protocol-registry/runner/src/production.rs", "protocol-registry/runner/src/references.rs",
    }
    ledgers: dict[str, set[str]] = {}

    for protocol in protocols:
        protocol_cases = [case for case in cases if case["registry_id"] == protocol["id"]]
        if not any(case["decision"] == "accept" for case in protocol_cases):
            fail(f"{protocol['id']} has no positive corpus case")
        if not any(case["decision"] == "reject" for case in protocol_cases):
            fail(f"{protocol['id']} has no negative corpus case")
        if any(not case["id"].startswith(protocol["corpus_prefix"]) for case in protocol_cases):
            fail(f"{protocol['id']} corpus prefix drift")
        implementations = protocol["independent_implementations"].split(";")
        if len(implementations) < 2 or not set(implementations) <= provenance_ids:
            fail(f"{protocol['id']} lacks two registered independent implementations")
        if not (root / "crates" / protocol["package"] / "Cargo.toml").is_file():
            fail(f"missing package {protocol['package']}")
        for field in ["source", "errata", "requirements", "api_snapshot"]:
            relative = protocol[field]
            if not (root / relative).is_file():
                fail(f"missing {field} for {protocol['id']}: {relative}")
            required_hashes.add(relative)
        ledgers[protocol["requirements"]] = requirement_ids(root / protocol["requirements"])
        source_hash = sha256(root / protocol["source"])
        errata_hash = sha256(root / protocol["errata"])
        for case in protocol_cases:
            if case["source_sha256"] != source_hash or case["errata_sha256"] != errata_hash:
                fail(f"{case['id']} is not bound to current source and errata bytes")
            if not set(case["requirement_ids"].split(";")) <= ledgers[protocol["requirements"]]:
                fail(f"{case['id']} cites an unknown requirement")
            if not set(case["provenance_ids"].split(";")) <= provenance_ids:
                fail(f"{case['id']} cites unknown provenance")

    registered_names = {row["public_symbol"] for row in configs}
    actual_names = public_configuration_names(root)
    if registered_names != actual_names:
        fail(
            f"public configuration registry drift; missing={sorted(actual_names - registered_names)}, "
            f"extra={sorted(registered_names - actual_names)}"
        )
    for config in configs:
        if config["corpus_case"] not in case_by_id:
            fail(f"{config['id']} cites missing corpus case")
        for field in ["source", "errata", "requirements"]:
            if not (root / config[field]).is_file():
                fail(f"{config['id']} cites missing {field}")
            required_hashes.add(config[field])
        expected = REFERENCE_LOCKS.get((config["reference"], config["version"]))
        if expected != config["checksum"]:
            fail(f"{config['id']} reference version/checksum is not pinned")
        if f"{config['reference']}-{config['version']}" not in provenance_ids:
            fail(f"{config['id']} reference provenance is missing")

    rfc4648_hash = sha256(root / "rfc/rfc4648.txt")
    rfc4648_errata = sha256(root / "rfc/rfc4648-errata.tsv")
    rfc4648_ids = requirement_ids(root / "rfc/rfc4648-requirements.json")
    for case in (case for case in cases if case["registry_id"] == "core-config"):
        if case["source_sha256"] != rfc4648_hash or case["errata_sha256"] != rfc4648_errata:
            fail(f"{case['id']} is not bound to locked RFC 4648 evidence")
        if not set(case["requirement_ids"].split(";")) <= rfc4648_ids:
            fail(f"{case['id']} cites unknown RFC 4648 requirements")
        if not set(case["provenance_ids"].split(";")) <= provenance_ids:
            fail(f"{case['id']} cites unknown reference provenance")

    model = (root / "protocol-registry/runner/src/model.rs").read_text(encoding="utf-8")
    for forbidden in ["base64_ng", "base64::", "base64ct"]:
        if forbidden in model:
            fail(f"independent model imports production/reference code: {forbidden}")
    validate_hash_inventory(root, required_hashes)
    print(f"protocol registry: {len(protocols)} claims, {len(configs)} configurations, and {len(cases)} cases ok")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"protocol registry: {error}", file=sys.stderr)
        raise SystemExit(1) from error
