#!/usr/bin/env python3
"""Exercise CI toolchain setup without installing or changing real toolchains."""
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
ACTIVE = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]


class InstallerTests(unittest.TestCase):
    def test_platform_paths_and_toolchain_forwarding(self):
        for platform in ("Windows", "Linux", "macOS"):
            with self.subTest(platform=platform), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                home = root / "home with spaces"
                binaries = home / ".cargo/bin"
                binaries.mkdir(parents=True)
                for name, body in {
                    "cargo": "printf 'native cargo proxy\\n'",
                    "rustc": f"printf 'rustc {ACTIVE} (fixture)\\n'",
                    "rustup": 'printf "%s\\n" "$@"',
                }.items():
                    tool = binaries / name
                    tool.write_text(f"#!/bin/sh\n{body}\n")
                    tool.chmod(0o700)
                path_file = root / "github-path"
                env = {"PATH": os.defpath, "HOME": str(home), "RUNNER_OS": platform,
                       "RUNNER_TEMP": str(root / "runner temp"), "GITHUB_PATH": str(path_file)}
                subprocess.run(["sh", str(ROOT / "scripts/ci_install_rust.sh")], cwd=ROOT,
                               env=env, check=True, capture_output=True, text=True, timeout=30)
                paths = path_file.read_text().splitlines()
                wrapper = root / "runner temp/base64-ng-rust-bin/cargo"
                if platform == "Windows":
                    self.assertEqual(paths, [str(binaries)])
                    self.assertFalse(wrapper.exists())
                else:
                    self.assertEqual(paths, [str(binaries), str(wrapper.parent)])
                    self.assertTrue(wrapper.is_file())
                env["PATH"] = os.pathsep.join([*reversed(paths), os.defpath])
                for prefix in ([], ["+1.90.0"]):
                    result = subprocess.run(["cargo", *prefix, "test", "argument with spaces"],
                                            env=env, check=True, capture_output=True,
                                            text=True, timeout=30)
                    if platform == "Windows":
                        self.assertEqual(result.stdout, "native cargo proxy\n")
                    else:
                        compiler = "1.90.0" if prefix else ACTIVE
                        self.assertEqual(result.stdout.splitlines(),
                                         ["run", compiler, "cargo", "test", "argument with spaces"])


if __name__ == "__main__":
    unittest.main()
