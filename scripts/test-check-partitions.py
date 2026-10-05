#!/usr/bin/env python3
"""Exercise the real shell dispatcher with logged, non-building gate stubs."""

from collections import Counter
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
DEVELOPMENT = (
    "baseline", "validation-policy", "ssse3-validation", "avx2-validation",
    "avx512-validation", "public-decode", "public-encode", "forwarding",
    "decode-composition", "borrowed-view", "incremental-bulk", "adapter-bulk",
    "in-place-bulk", "companion-features", "neon-validation", "public-api",
)


def dispatch(args, fail="", source=None):
    if source is None:
        source = (ROOT / "scripts/checks.sh").read_text()
    with tempfile.TemporaryDirectory(prefix="base64-check-partitions-") as directory:
        work = Path(directory)
        # Keep even the license output of the real dispatcher inside the fixture.
        source = source.replace("/tmp/base64-ng-cargo-license.json",
                                str(work / "license.json"))
        script = work / "checks.sh"
        script.write_text(source)
        stub = '''#!/bin/sh
printf '%s\\n' "$0 $*" >> "$CHECK_LOG"
if [ "$0" = "$FAIL_COMMAND" ]; then exit 17; fi
printf '{}\\n'
'''
        for name in set(re.findall(r"scripts/[\w.-]+\.(?:sh|py)", source)) | {
                "bin/cargo", "bin/python3"}:
            path = work / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(stub)
            path.chmod(0o700)
        log = work / "calls.log"
        env = dict(os.environ, PATH=str(work / "bin") + os.pathsep + os.environ["PATH"],
                   CHECK_LOG=str(log), FAIL_COMMAND=fail)
        result = subprocess.run(["sh", str(script), *args], cwd=work, env=env,
                                capture_output=True, text=True, timeout=20)
        calls = [line.replace(str(work / "bin") + "/", "").rstrip()
                 for line in log.read_text().splitlines()] if log.exists() else []
        return result, calls


class PartitionTests(unittest.TestCase):
    def test_partitions_cover_every_default_command_without_overlap(self):
        full, full_calls = dispatch([])
        core, core_calls = dispatch(["--core"])
        development, development_calls = dispatch(["--development"])
        for result in (full, core, development):
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(Counter(full_calls), Counter(core_calls + development_calls))
        self.assertFalse(set(core_calls) & set(development_calls))
        self.assertEqual(development_calls,
                         [f"scripts/check-2.1-{name}.sh" for name in DEVELOPMENT])
        self.assertIn("scripts/check-2.1-wasm-validation.sh", core_calls)
        self.assertIn("cargo test --all-targets --all-features", core_calls)
        explicit, calls = dispatch(["--all"])
        self.assertEqual(explicit.returncode, 0, explicit.stderr)
        self.assertEqual(calls, full_calls)

    def test_invalid_arguments_fail_before_any_gate(self):
        for args in (["--unknown"], ["--core", "--development"], [""]):
            with self.subTest(args=args):
                result, calls = dispatch(args)
                self.assertEqual(result.returncode, 2)
                self.assertFalse(calls)

    def test_failures_stop_each_partition_and_the_full_suite(self):
        for mode, failure in (("--core", "scripts/validate-release-metadata.sh"),
                              ("--development", "scripts/check-2.1-baseline.sh"),
                              ("--all", "scripts/check-2.1-baseline.sh")):
            with self.subTest(mode=mode):
                result, calls = dispatch([mode], fail=failure)
                self.assertEqual(result.returncode, 17)
                self.assertEqual(calls[-1], failure)

if __name__ == "__main__":
    unittest.main()
