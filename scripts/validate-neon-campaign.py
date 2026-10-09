#!/usr/bin/env python3
"""Validate both native NEON bundles for one exact release campaign."""

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import re
import stat


VALIDATOR = Path(__file__).with_name("validate-neon-admission-bundle.py")
SPEC = importlib.util.spec_from_file_location("neon_admission", VALIDATOR)
if SPEC is None or SPEC.loader is None:
    raise SystemExit("NEON campaign: cannot load admission validator")
ADMISSION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ADMISSION)


def validate(root: Path, source: str) -> None:
    if re.fullmatch(r"[0-9a-f]{40}", source) is None:
        raise SystemExit("NEON campaign: expected source must be a full commit")
    for parent in (root, *root.parents):
        if parent.is_symlink():
            raise SystemExit("NEON campaign: symbolic-link directory")
    if not stat.S_ISDIR(root.lstat().st_mode):
        raise SystemExit("NEON campaign: root is not a regular directory")
    if {path.name for path in root.iterdir()} != {"apple-silicon", "aarch64-linux"}:
        raise SystemExit("NEON campaign: unexpected campaign root inventory")
    for platform in ("apple-silicon", "aarch64-linux"):
        bundle = root / platform
        if not stat.S_ISDIR(bundle.lstat().st_mode):
            raise SystemExit("NEON campaign: missing regular bundle directory")
        if {path.name for path in bundle.iterdir()} != ADMISSION.FILES:
            raise SystemExit("NEON campaign: unexpected bundle inventory")
        for path in bundle.iterdir():
            info = path.lstat()
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                raise SystemExit("NEON campaign: non-regular or linked artifact")
        manifest = ADMISSION.parse_manifest(bundle / "MANIFEST.txt")
        if manifest["source_commit"] != source:
            raise SystemExit("NEON campaign: bundle does not match the exact campaign")
        ADMISSION.validate(bundle, platform, allow_runtime_drift=False)
    print(f"NEON campaign: both native platforms verified for {source}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("--source", required=True)
    arguments = parser.parse_args()
    try:
        validate(arguments.root, arguments.source)
    except (OSError, ValueError) as error:
        raise SystemExit(f"NEON campaign: {error}") from error


if __name__ == "__main__":
    main()
