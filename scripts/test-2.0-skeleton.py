"""Regression checks for the exact nested test-oracle allowlist and its gates."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix="base64-ng-skeleton-") as directory:
    fixture = Path(directory)
    shutil.copytree(root / "src", fixture / "src")
    script = fixture / "validate.sh"
    shutil.copyfile(root / "scripts/validate-2.0-skeleton.sh", script)
    bin_dir = fixture / "bin"
    bin_dir.mkdir()
    cargo = bin_dir / "cargo"
    cargo.write_text("#!/bin/sh\nset -eu\ntest \"$*\" = \"test --lib v2::\"\ntouch cargo-reached\n")
    cargo.chmod(0o755)
    env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ["PATH"])

    def check(accepted):
        marker = fixture / "cargo-reached"
        marker.unlink(missing_ok=True)
        result = subprocess.run(["sh", str(script)], cwd=fixture, env=env,
                                capture_output=True, text=True)
        if (result.returncode == 0) != accepted or marker.exists() != accepted:
            raise AssertionError(result.stdout + result.stderr)

    check(True)
    for parent in ("src/v2/ordinary_decode.rs", "src/v2/ordinary_decode/ssse3_candidate.rs",
                   "src/v2/ordinary_decode/avx2_candidate.rs", "src/v2/ordinary_decode/avx512_candidate.rs"):
        path = fixture / parent
        original = path.read_text()
        declaration = "#[cfg(test)]\nmod tests;"
        assert declaration in original
        path.write_text(original.replace(declaration, "mod tests;"))
        check(False)
        path.write_text(original)
    for parent in ("src/v2/ordinary_decode/avx2_candidate.rs", "src/v2/ordinary_decode/avx512_candidate.rs"):
        path = fixture / parent
        original = path.read_text()
        declaration = "#[cfg(test)]\nmod benchmark;"
        assert declaration in original
        path.write_text(original.replace(declaration, "mod benchmark;"))
        check(False)
        path.write_text(original)
    # An arbitrary nested tests.rs must not inherit an exemption by filename.
    for name in ("src/v2/production_oracle.rs", "src/v2/unreviewed/tests.rs"):
        path = fixture / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("use crate::v2::rfc4648_oracle;\n")
        check(False)
        path.unlink()
    check(True)
print("2.0 skeleton mutations: reviewed test imports accepted; missing gates and unlisted imports rejected")
