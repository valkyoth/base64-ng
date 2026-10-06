#!/usr/bin/env python3
"""Exercise native capture parsing without claiming Windows execution."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("windows_measure", Path(__file__).with_name("measure-windows-public-api.py"))
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)


class MeasurementTests(unittest.TestCase):
    def run_capture(self, allocation=0, missing=False):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "probe.exe"
            binary.write_bytes(b"fixture, never executed")
            def output(command, **_):
                if command[1] == "list":
                    return "canonical\nhistorical\nscalar\n" + ("" if missing else "canonical-reference\nhistorical-reference\n")
                size, profile, rounds = int(command[4]), command[3], command[7]
                encoded = (size + 2) // 3 * 4 if profile.endswith("p") else (size * 8 + 5) // 6
                return ("elapsed_ns,iterations,raw_bytes,encoded_bytes,allocations,encode_capability,decode_capability\n"
                        f"1000,{rounds},{size},{encoded},{allocation},scalar,scalar\n")
            with patch.object(measure.subprocess, "check_output", side_effect=output):
                return measure.capture(binary, sizes=(32,))

    def test_pairs_units_sample_counts_and_unavailable_backends(self):
        report = self.run_capture()
        self.assertEqual(len(report["rows"]), 720)
        self.assertEqual(len(report["summary"]), 24)
        self.assertTrue(all(row["samples"] == 15 for row in report["summary"]))
        self.assertEqual(len(report["unavailable_backends"]), 3)
        self.assertEqual(report["rows"][0]["side"], "baseline")
        self.assertEqual(report["rows"][2]["side"], "candidate")

    def test_bad_allocation_and_missing_policy_fail(self):
        for kwargs in ({"allocation": 1}, {"missing": True}):
            with self.assertRaises(ValueError):
                self.run_capture(**kwargs)


if __name__ == "__main__":
    unittest.main()
