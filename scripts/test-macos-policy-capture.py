#!/usr/bin/env python3
"""Test native build isolation and provenance with inert compiler fixtures."""
import importlib.util
import json
from pathlib import Path
import runpy
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("mac_capture", Path(__file__).with_name("capture-2.1-macos-policy.py"))
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)


class CaptureTests(unittest.TestCase):
    def test_old_python_exits_before_importing_tomllib_or_starting_capture(self):
        with patch.object(sys, "version_info", (3, 9, 6)), \
             patch.dict(sys.modules, {"tomllib": None}), \
             patch.object(tempfile, "TemporaryDirectory") as temporary:
            with self.assertRaisesRegex(SystemExit, r"Python 3\.12\+ is required"):
                runpy.run_path(str(Path(__file__).with_name("capture-2.1-macos-policy.py")),
                               run_name="__main__")
            temporary.assert_not_called()

    def exercise(self, fail=False, dirty=False):
        compare = capture.module("compare_test", "compare-2.1-public-api.py")
        measure = capture.module("measure_test", "measure-2.1-native-policy.py")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / "bin"
            tools.mkdir()
            for name in ("rustc", "cargo"):
                (tools / name).write_bytes(b"not executable")
            destination = root / "capture"
            builds = []

            def execute(command, **kwargs):
                if command[:3] == ["git", "status", "--porcelain"]:
                    return b" M source.rs" if dirty else b""
                if command[:3] == ["git", "rev-parse", "HEAD"]:
                    return b"a" * 40
                if "which" in command:
                    return str(tools / "rustc").encode()
                if command[-1] == "-Vv":
                    return b"rustc fixture"
                self.assertIn("--locked", command)
                self.assertIn("--offline", command)
                env = kwargs["env"]
                self.assertEqual(env["CARGO_INCREMENTAL"], "0")
                for key in ("RUSTC_WRAPPER", "CARGO_BUILD_BUILD_DIR", "AWS_SECRET_ACCESS_KEY"):
                    self.assertNotIn(key, env)
                target = Path(command[command.index("--target-dir") + 1])
                self.assertEqual(target.parent.stat().st_mode & 0o777, 0o700)
                builds.append(target)
                if fail:
                    raise RuntimeError("compiler failed")
                binary = target / "release/base64-ng-public-api-perf"
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"built fixture")
                return b""

            def lock(tree):
                path = tree / "Cargo.lock"
                path.write_text("version = 4\n")
                return path

            argv = ["capture", "--trusted-revisions", "--output", str(destination), "--features", "simd"]
            with patch.object(capture.sys, "argv", argv), patch.object(capture.sys, "platform", "darwin"), \
                 patch.object(capture, "bounded", side_effect=execute), \
                 patch.object(capture.shutil, "which", return_value="/fixture/rustup"), \
                 patch.object(capture, "module", side_effect=[compare, measure]), \
                 patch.object(compare, "extract_revision"), patch.object(compare, "install_harness"), \
                 patch.object(compare, "prepare_lock", side_effect=lock), \
                 patch.object(measure, "capture", return_value={"scope": "fixture"}):
                if fail or dirty:
                    with self.assertRaises((ValueError, RuntimeError)):
                        capture.main()
                else:
                    capture.main()
            self.assertTrue(all(not path.parent.exists() for path in builds))
            if dirty:
                self.assertFalse(destination.exists())
                return
            report = json.loads((destination / "manifest.json").read_text())
            self.assertEqual(report["status"], "failed" if fail else "passed")
            if not fail:
                self.assertEqual(len(builds), 3)
                self.assertEqual(len(set(builds)), 3)
                self.assertEqual(set(report["captures"]), {"simd-revisions.json", "simd-controls.json"})
                self.assertEqual(set(report["locks_sha256"]), {"baseline-Cargo.lock", "candidate-Cargo.lock"})
                self.assertNotIn(directory, json.dumps(report))

    def test_fresh_private_builds_and_bound_provenance(self):
        self.exercise()

    def test_failed_build_cleans_up_without_pass_claim(self):
        self.exercise(fail=True)

    def test_dirty_sources_fail_before_capture(self):
        self.exercise(dirty=True)


if __name__ == "__main__":
    unittest.main()
