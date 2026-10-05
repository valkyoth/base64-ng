#!/usr/bin/env python3
"""Regression checks for the example test's build/execution boundary."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
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
    def test_build_is_private_and_selects_the_cargo_reported_executable(self):
        messages = [artifact(name="other"), artifact(kind="lib"), artifact(),
                    {"reason": "build-finished", "success": True}]
        result = subprocess.CompletedProcess([], 0, "\n".join(map(json.dumps, messages)))
        with patch.object(runner.subprocess, "run", return_value=result) as run:
            self.assertEqual(runner.build_example(Path("/repo"), "1.90.0"),
                             "/build/stream_file")
        command = run.call_args.args[0]
        self.assertEqual(command[:3], ["cargo", "+1.90.0", "build"])
        self.assertIn("--message-format=json", command)
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
                        runner.build_example(Path("/repo"), "1.90.0")

    def test_build_failure_is_not_accepted_as_a_refusal_test(self):
        with patch.object(runner.subprocess, "run",
                          side_effect=subprocess.CalledProcessError(1, ["cargo"])):
            with self.assertRaises(subprocess.CalledProcessError):
                runner.build_example(Path("/repo"), "1.90.0")

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
