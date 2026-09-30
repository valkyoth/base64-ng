#!/usr/bin/env python3
"""Prove development-release checks reject bad results even under optimization."""

import os
from pathlib import Path
import subprocess
import sys
import unittest


ROOT = Path(__file__).resolve().parents[1]
CHILD = '''
import runpy
import subprocess
import sys

sys.path.insert(0, "scripts")
checks = runpy.run_path("scripts/test-development-release.py")
case = sys.argv[1]
if case == "wrong-error":
    def fail():
        raise RuntimeError("unrelated rejection")
    checks["reject"]("expected rejection", fail)
elif case == "no-error":
    checks["reject"]("expected rejection", lambda: None)
else:
    if case == "rust-plan":
        checks["release"].publish_plan = lambda plan: ("unexpected-package",)
    elif case == "npm-plan":
        checks["release"].npm_plan_output = lambda plan: "publish=true"
    elif case == "exit-code":
        subprocess.run = lambda *args, **kwargs: subprocess.CompletedProcess(
            args[0], 0, "", "development-blocked")
    elif case == "diagnostic":
        subprocess.run = lambda *args, **kwargs: subprocess.CompletedProcess(
            args[0], 1, "", "unrelated failure")
    checks["main"]()
'''


class OptimizationTests(unittest.TestCase):
    def test_valid_and_mutated_checks_in_every_mode(self):
        failures = {
            "wrong-error": "unrelated rejection",
            "no-error": "accepted invalid fixture",
            "rust-plan": "development plan publishes crates",
            "npm-plan": "npm plan is not blocked",
            "exit-code": "unexpected publication exit status",
            "diagnostic": "unexpected publication diagnostic",
        }
        for flags, optimize in (([], None), (["-O"], None), (["-OO"], None), ([], "2")):
            env = dict(os.environ)
            env.pop("PYTHONOPTIMIZE", None)
            if optimize is not None:
                env["PYTHONOPTIMIZE"] = optimize
            for case in ("valid", *failures):
                with self.subTest(flags=flags, optimize=optimize, case=case):
                    result = subprocess.run(
                        [sys.executable, *flags, "-c", CHILD, case], cwd=ROOT,
                        env=env, capture_output=True, text=True, timeout=60,
                    )
                    if case == "valid":
                        self.assertEqual(result.returncode, 0, result.stderr)
                        self.assertIn("fail closed", result.stdout)
                    else:
                        self.assertNotEqual(result.returncode, 0, result.stdout)
                        self.assertIn("AssertionError", result.stderr)
                        self.assertIn(failures[case], result.stderr)
                        self.assertNotIn("fail closed", result.stdout)


if __name__ == "__main__":
    unittest.main()
