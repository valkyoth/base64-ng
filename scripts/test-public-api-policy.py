#!/usr/bin/env python3
"""Policy capture regression tests; no hardware measurements in CI."""
from pathlib import Path
import subprocess
import sys
import unittest

from public_api_policy import SURFACES, iterations, operation, policy_cases


class PolicyTests(unittest.TestCase):
    def test_surface_boundaries_profiles_and_modes(self):
        cases = list(policy_cases(SURFACES))
        self.assertEqual(len(cases), len({tuple(case) for case in cases}))
        self.assertEqual({case[0] for case in cases}, set(SURFACES))
        self.assertEqual({case[2] for case in cases}, {"sp", "su", "up", "uu"})
        for name in ("canonical", "historical"):
            self.assertTrue({"0", "3", "383", "384", "3071", "3072", "1048576"} <=
                            {case[3] for case in cases if case[0] == name})
        for name in ("incremental", "sync", "bytes", "tokio"):
            self.assertEqual({case[5] for case in cases if case[0] == name}, {"1", "7", "4096"})
        for case in cases:
            count = iterations(case)
            if case[-1] == "cold":
                self.assertEqual(count, 1)
            else:
                self.assertTrue(8 <= count <= 16384)
            if case[0] == "validate" or case[-1].startswith("invalid-"):
                self.assertEqual(case[1], "decode")
            self.assertEqual(operation(case, "baseline"), case[0])
            self.assertEqual(operation(case, "candidate"), case[0])

    def test_controls_do_not_claim_unavailable_backends(self):
        names = ["canonical", "historical", "canonical-reference", "historical-reference",
                 "scalar", "base64", "base64ct"]
        with self.assertRaises(ValueError):
            list(policy_cases(names[:-5], True))
        self.assertFalse(any("avx" in case[0] for case in policy_cases(names, True)))
        cases = list(policy_cases(names + ["avx2", "avx512-vbmi"], True))
        self.assertTrue(any(case[0] == "avx2->avx512-vbmi" for case in cases))
        for case in cases:
            left, right = case[0].split("->")
            self.assertEqual(operation(case, "baseline"), left)
            self.assertEqual(operation(case, "candidate"), right)
            if "reference" in left:
                self.assertEqual(case[1], "decode")

    def test_cli_rejects_weak_policy_or_unsupported_control_capture(self):
        script = Path(__file__).with_name("compare-2.1-public-api.py")
        for args in (["--policy-matrix"], ["--controls"],
                     ["--controls", "--policy-matrix", "--samples", "15", "--features", "core"]):
            result = subprocess.run([sys.executable, str(script), "--output", "unused", *args],
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 2)
            self.assertIn("error:", result.stderr)


if __name__ == "__main__":
    unittest.main()
