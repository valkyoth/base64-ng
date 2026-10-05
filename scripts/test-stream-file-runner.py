#!/usr/bin/env python3
"""Regression checks for the example test's build/execution boundary."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import stat
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "stream_file_example", Path(__file__).with_name("test-stream-file-example.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def artifact(name="stream_file", kind="example", executable="/build/stream_file"):
    return {"reason": "compiler-artifact", "target": {"name": name, "kind": [kind]},
            "executable": executable}


class RunnerTests(unittest.TestCase):
    def test_main_uses_fresh_private_builds_and_keeps_them_alive_until_execution(self):
        targets = []

        def build(root, toolchain, target_dir):
            self.assertEqual(toolchain, "1.90.0")
            self.assertFalse(target_dir.exists())
            self.assertTrue(target_dir.parent.is_dir())
            if os.name == "posix":
                self.assertEqual(stat.S_IMODE(target_dir.parent.stat().st_mode), 0o700)
            target_dir.mkdir()
            targets.append(target_dir)
            return str(target_dir / "stream_file")

        def execute(executable, source, destination, root, *, success):
            self.assertTrue(Path(executable).parent.is_dir())
            if success:
                destination.write_bytes(b"Zm9vYmFyIQ==")
                destination.chmod(0o600)

        with patch.object(runner, "build_example", side_effect=build), \
                patch.object(runner, "run_example", side_effect=execute), \
                patch("sys.argv", ["test-stream-file-example.py", "1.90.0"]), \
                patch("builtins.print"):
            runner.main()
            runner.main()
        self.assertEqual(len(targets), 2)
        self.assertNotEqual(targets[0], targets[1])
        self.assertTrue(all(not target.parent.exists() for target in targets))

    def test_failed_build_also_cleans_private_directory(self):
        targets = []

        def build(root, toolchain, target_dir):
            targets.append(target_dir)
            target_dir.mkdir()
            raise RuntimeError("build failed")

        with patch.object(runner, "build_example", side_effect=build), \
                patch("sys.argv", ["test-stream-file-example.py", "1.90.0"]):
            with self.assertRaisesRegex(RuntimeError, "build failed"):
                runner.main()
        self.assertEqual(len(targets), 1)
        self.assertFalse(targets[0].parent.exists())

    def test_build_is_private_and_selects_the_cargo_reported_executable(self):
        messages = [artifact(name="other"), artifact(kind="lib"), artifact(),
                    {"reason": "build-finished", "success": True}]
        result = subprocess.CompletedProcess([], 0, "\n".join(map(json.dumps, messages)))
        inherited = dict(CARGO_TARGET_DIR="/old-target", CARGO_BUILD_BUILD_DIR="/old-build",
                         RUSTC_WRAPPER="cached-rustc", RUSTC_WORKSPACE_WRAPPER="cached-workspace")
        with patch.dict(os.environ, inherited), \
                patch.object(runner.subprocess, "run", return_value=result) as run:
            self.assertEqual(runner.build_example(Path("/repo"), "1.90.0", Path("/build")),
                             str(Path("/build/stream_file").resolve()))
        command = run.call_args.args[0]
        self.assertEqual(command[:3], ["cargo", "+1.90.0", "build"])
        self.assertIn("--message-format=json", command)
        self.assertEqual(command[-2:], ["--target-dir", str(Path("/build").resolve())])
        env = run.call_args.kwargs["env"]
        self.assertEqual(env["CARGO_TARGET_DIR"], str(Path("/build").resolve()))
        self.assertEqual(env["CARGO_BUILD_BUILD_DIR"], str(Path("/build/build").resolve()))
        self.assertEqual(env["RUSTC_WRAPPER"], "")
        self.assertEqual(env["RUSTC_WORKSPACE_WRAPPER"], "")
        self.assertTrue(run.call_args.kwargs["check"])
        if os.name == "posix":
            self.assertEqual(run.call_args.kwargs["umask"], 0o077)

    def test_missing_or_ambiguous_executable_fails_closed(self):
        for messages in [[], [artifact(executable=None)], [artifact(name="other")],
                         [artifact(), artifact()]]:
            with self.subTest(messages=messages):
                result = subprocess.CompletedProcess([], 0, "\n".join(map(json.dumps, messages)))
                with patch.object(runner.subprocess, "run", return_value=result):
                    with self.assertRaises(AssertionError):
                        runner.build_example(Path("/repo"), "1.90.0", Path("/build"))

    def test_build_failure_is_not_accepted_as_a_refusal_test(self):
        with patch.object(runner.subprocess, "run",
                          side_effect=subprocess.CalledProcessError(1, ["cargo"])):
            with self.assertRaises(subprocess.CalledProcessError):
                runner.build_example(Path("/repo"), "1.90.0", Path("/build"))

    def test_artifact_outside_private_target_is_rejected(self):
        result = subprocess.CompletedProcess([], 0, json.dumps(artifact(executable="/old/example")))
        with patch.object(runner.subprocess, "run", return_value=result):
            with self.assertRaises(ValueError):
                runner.build_example(Path("/repo"), "1.90.0", Path("/build"))

    @unittest.skipUnless(os.name == "posix", "Unix symlink regression")
    def test_artifact_symlink_to_old_cache_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            target = work / "target"
            target.mkdir()
            old = work / "old-executable"
            old.touch()
            link = target / "stream_file"
            link.symlink_to(old)
            result = subprocess.CompletedProcess([], 0, json.dumps(artifact(executable=str(link))))
            with patch.object(runner.subprocess, "run", return_value=result):
                with self.assertRaises(ValueError):
                    runner.build_example(work, "1.90.0", target)

    def test_only_direct_example_execution_has_the_hostile_umask(self):
        result = subprocess.CompletedProcess([], 0, "", "")
        with patch.object(runner.subprocess, "run", return_value=result) as run:
            runner.run_example("/build/stream_file", Path("input"), Path("output"),
                               Path("/repo"), success=True)
        self.assertEqual(run.call_args.args[0], ["/build/stream_file", "input", "output"])
        if os.name == "posix":
            self.assertEqual(run.call_args.kwargs["umask"], 0)


if __name__ == "__main__":
    unittest.main()
