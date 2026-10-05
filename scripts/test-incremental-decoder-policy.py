#!/usr/bin/env python3
"""Mutation checks for source guards after splitting the decoder update module."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
CHECKS = (
    "scripts/check-2.0-incremental-padded-decoder.sh",
    "scripts/check-2.0-incremental-decoder-finalization.sh",
)
UPDATE = "src/v2/incremental_decoder/update.rs"
FILES = (
    *CHECKS,
    UPDATE,
    "src/v2/incremental_decoder.rs",
    "src/v2/lifecycle.rs",
    "src/kani_proofs.rs",
    "docs/2.0_INCREMENTAL_PADDED_DECODER.md",
    "docs/2.0_INCREMENTAL_DECODER_FINALIZATION.md",
)


def main():
    with tempfile.TemporaryDirectory(prefix="base64-incremental-policy-") as directory:
        root = Path(directory)
        for name in FILES:
            destination = root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, destination)
        # Only the source guards are under test; the real Rust matrix runs separately.
        cargo = root / "bin/cargo"
        cargo.parent.mkdir()
        cargo.write_text("#!/bin/sh\nexit 0\n")
        cargo.chmod(0o755)
        env = dict(os.environ, PATH=f"{cargo.parent}{os.pathsep}{os.environ['PATH']}")
        update = root / UPDATE
        original = update.read_text()

        def check(script, accepted, diagnostic=""):
            result = subprocess.run(["sh", script], cwd=root, env=env,
                                    text=True, capture_output=True, check=False)
            assert (result.returncode == 0) == accepted, result.stdout + result.stderr
            assert diagnostic in result.stdout + result.stderr

        for script in CHECKS:
            check(script, True)
            for forbidden in ("unsafe", "alloc::", ".unwrap(", "panic!"):
                update.write_text(original + f"\n// injected {forbidden}\n")
                check(script, False, "core gained allocation, panic, unsafe, or Drop")
            update.unlink()
            check(script, False)
            update.write_text(original)
        update.write_text(original.replace("InputError::TrailingData", "InputError::InvalidLength"))
        check(CHECKS[0], False, "implementation is missing: InputError::TrailingData")
    print("incremental decoder source guards: split module and forbidden-code mutations passed")


if __name__ == "__main__":
    main()
