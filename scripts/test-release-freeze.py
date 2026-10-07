#!/usr/bin/env python3
"""Reject missing user examples and repository-only published payloads."""

import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


path = Path(__file__).with_name("check-2.1-package-inventory.py")
spec = importlib.util.spec_from_file_location("inventory", path)
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.files = ["Cargo.toml", "README.md", "src/lib.rs", "LICENSE-MIT",
                      "LICENSE-APACHE", "examples/decode_policy.rs", "examples/stream_file.rs"]

    def test_valid(self):
        inventory.validate_inventory("base64-ng", self.files)

    def test_missing_and_duplicate(self):
        for files in ([], self.files[:-1], self.files + ["src/lib.rs"]):
            with self.assertRaises(ValueError):
                inventory.validate_inventory("base64-ng", files)

    def test_process_payload_and_path_escape(self):
        for name in [f"{directory}/payload" for directory in inventory.REPOSITORY_ONLY] + [
            "", "/absolute", "../escape", "src/../../escape", "src\\escape",
        ]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                inventory.validate_inventory("base64-ng", self.files + [name])

    def test_package_ceiling(self):
        with self.assertRaises(ValueError):
            inventory.validate_inventory("base64-ng", self.files +
                                         [f"src/extra{i}.rs" for i in range(238)])


class GateTests(unittest.TestCase):
    def test_strict_release_routing_and_source_guards(self):
        root = path.resolve().parents[1]
        with tempfile.TemporaryDirectory() as raw:
            fixture = Path(raw)
            scripts = fixture / "scripts"
            scripts.mkdir()
            for name in ("stable_release_gate.sh", "evidence-source.sh"):
                shutil.copy2(root / "scripts" / name, scripts / name)
            (fixture / "release-crates.toml").write_text('[release]\npolicy = "synced-family"\n')
            (fixture / "Cargo.lock").write_text("fixture\n")
            bin_dir = fixture / "bin"
            bin_dir.mkdir()
            commands = {
                bin_dir / "cargo": "exit 1\n",
                bin_dir / "git": 'case "$1" in rev-parse) printf "%040d\\n" 1;; '
                                 'status) printf "%s" "${TEST_DIRTY:-}";; *) exit 99;; esac\n',
                scripts / "checks.sh": "echo standard-gate-fixture\n",
                scripts / "validate-2.0-checkpoint-record.py": "echo legacy-checkpoint-fixture\n",
                scripts / "check_miri.sh": "exit 81\n",
            }
            for command, body in commands.items():
                command.write_text("#!/bin/sh\n" + body)
                command.chmod(0o700)
            env = {key: value for key, value in os.environ.items()
                   if not key.startswith("BASE64_NG_")}
            env["PATH"] = f"{bin_dir}:{env['PATH']}"
            gate = ["sh", str(scripts / "stable_release_gate.sh"), "release"]
            for version in ("2.0.0", "2.1.0"):
                (fixture / "Cargo.toml").write_text(f'[package]\nversion = "{version}"\n')
                result = subprocess.run(gate, cwd=fixture, env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 81, result.stderr)
                self.assertEqual("legacy-checkpoint-fixture" in result.stdout, version == "2.0.0")
                self.assertIn("standard-gate-fixture", result.stdout)
            for extra, message in (
                ({"TEST_DIRTY": " M src/lib.rs", "BASE64_NG_ALLOW_DIRTY_EVIDENCE": "1"},
                 "dirty tree"),
                ({"BASE64_NG_EVIDENCE_SIGNING_KEY": "never-expose"}, "signing key"),
            ):
                result = subprocess.run(gate, cwd=fixture, env=dict(env, **extra),
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)
                self.assertNotIn("standard-gate-fixture", result.stdout)


if __name__ == "__main__":
    unittest.main()
