#!/usr/bin/env python3
"""Validate the complete Kani harness inventory and resource classes."""

from __future__ import annotations

import csv
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "kani/harnesses.tsv"
PROOF = re.compile(r"^\s*#\[kani::proof\]\s*$")
FUNCTION = re.compile(r"^\s*fn\s+([A-Za-z0-9_]+)\s*\(")
ALLOWED_SETS = {"normal", "advanced", "exploratory"}
ADVANCED_MODULES = {
    "src/kani_assurance_proofs.rs",
    "src/kani_secret_encode_proofs.rs",
    "src/kani_secret_proofs.rs",
}
REVIEWED_EXPLORATORY = {
    "advanced_wrapped_standard_decode_clear_tail_clears_output_on_error",
    "advanced_wrapped_standard_decode_slice_returns_written_within_output",
    "incremental_decode_matches_rfc_known_answer_all_tail_lengths",
    "in_place_encode_decode_matches_rfc_known_answer_all_tail_lengths",
    "in_place_validation_error_rolls_back_complete_buffer",
    "borrowed_secret_frame_writes_public_output_only_after_valid_gate",
}


def fail(message: str) -> None:
    print(f"Kani inventory: {message}", file=sys.stderr)
    raise SystemExit(1)


def source_harnesses() -> dict[str, tuple[str, bool]]:
    found: dict[str, tuple[str, bool]] = {}
    for path in sorted((ROOT / "src").glob("kani*_proofs.rs")):
        relative = path.relative_to(ROOT).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines):
            if not PROOF.match(line):
                continue
            function_index = index + 1
            while function_index < len(lines) and lines[function_index].lstrip().startswith("#["):
                function_index += 1
            if function_index >= len(lines):
                fail(f"proof attribute without function in {relative}:{index + 1}")
            match = FUNCTION.match(lines[function_index])
            if match is None:
                fail(f"proof attribute without named function in {relative}:{index + 1}")
            name = match.group(1)
            if name in found:
                fail(f"duplicate proof function {name}")
            attribute_start = index
            while attribute_start > 0 and lines[attribute_start - 1].lstrip().startswith("#["):
                attribute_start -= 1
            attributes = "\n".join(lines[attribute_start:function_index])
            found[name] = (relative, "base64_ng_kani_advanced" in attributes)
    return found


def manifest_rows() -> dict[str, dict[str, str]]:
    with MANIFEST.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        expected = ["harness", "set", "source", "claim"]
        if reader.fieldnames != expected:
            fail(f"manifest columns must be {expected}")
        rows: dict[str, dict[str, str]] = {}
        for row in reader:
            name = row["harness"]
            if not name or name in rows:
                fail(f"empty or duplicate manifest harness {name!r}")
            if row["set"] not in ALLOWED_SETS:
                fail(f"{name} has unknown set {row['set']!r}")
            if not row["claim"].strip():
                fail(f"{name} has no scoped claim")
            rows[name] = row
        return rows


def main() -> None:
    source = source_harnesses()
    manifest = manifest_rows()
    if source.keys() != manifest.keys():
        missing = sorted(source.keys() - manifest.keys())
        stale = sorted(manifest.keys() - source.keys())
        fail(f"source/manifest drift; missing={missing}, stale={stale}")

    counts = {name: 0 for name in ALLOWED_SETS}
    for name, row in manifest.items():
        source_path, locally_advanced = source[name]
        if row["source"] != source_path:
            fail(f"{name} source is {source_path}, manifest says {row['source']}")
        advanced = locally_advanced or source_path in ADVANCED_MODULES
        if row["set"] == "normal" and advanced:
            fail(f"{name} is advanced-gated but classified normal")
        if row["set"] in {"advanced", "exploratory"} and not advanced:
            fail(f"{name} lacks an advanced cfg or advanced module gate")
        if row["set"] == "exploratory" and name not in REVIEWED_EXPLORATORY:
            fail(f"only explicitly reviewed harnesses may remain exploratory: {name}")
        counts[row["set"]] += 1

    print(
        "Kani inventory: "
        f"{counts['normal']} normal, {counts['advanced']} advanced, "
        f"{counts['exploratory']} exploratory harnesses ok"
    )


if __name__ == "__main__":
    main()
