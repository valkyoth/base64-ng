#!/usr/bin/env python3
"""Native capture parser tests using inert files, never candidate execution."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("native_policy", Path(__file__).with_name("measure-2.1-native-policy.py"))
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)


class MeasurementTests(unittest.TestCase):
    def run_capture(self, fault=None, controls=False):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "probe"
            binary.write_bytes(b"inert fixture")
            calls = []
            cases = [["canonical-reference->canonical" if controls else "canonical", "decode",
                      "su", "32", "random", "4096", mode] for mode in ("warm", "cold", "invalid-0")]

            def execute(command, **_):
                calls.append(command)
                if command[1] == "list":
                    return b"canonical\nhistorical\ncanonical-reference\nhistorical-reference\n"
                if command[1] == "diagnostics":
                    return b"different" if fault == "diagnostics" and len(calls) == 4 else b"diagnostics"
                if fault == "failure":
                    raise RuntimeError("failed benchmark")
                if fault == "changed":
                    binary.write_bytes(b"changed fixture")
                rounds = command[7] if fault != "iterations" else "0"
                header = "elapsed_ns,iterations,raw_bytes,encoded_bytes,allocations,encode_capability,decode_capability\n"
                output = header + f"1000,{rounds},32,43,0,scalar,scalar\n"
                return (output + output if fault == "duplicate" else output).encode()

            with patch.object(measure, "bounded", side_effect=execute), patch.object(measure, "policy_cases", return_value=cases):
                report = measure.capture(binary, binary, controls=controls)
            return report, calls[4:]

    def test_alternating_pairs_cold_calls_and_rejection_units(self):
        report, calls = self.run_capture()
        self.assertEqual(len(report["rows"]), 90)
        self.assertEqual(len(report["summary"]), 3)
        self.assertTrue(all(row["samples"] == 15 for row in report["summary"]))
        self.assertEqual([r["side"] for r in report["rows"][:4]],
                         ["baseline", "candidate", "candidate", "baseline"])
        self.assertTrue(all(command[7] == "1" for command in calls if command[-1] == "cold"))
        self.assertIsNone(report["summary"][-1]["payload_gib_s"])
        self.assertNotIn(str(Path.home()), str(report))

    def test_reference_pair_uses_distinct_operations(self):
        _, calls = self.run_capture(controls=True)
        self.assertEqual([c[1] for c in calls[:4]],
                         ["canonical-reference", "canonical", "canonical", "canonical-reference"])

    def test_invalid_outputs_and_changed_binary_fail_closed(self):
        for fault in ("diagnostics", "failure", "changed", "iterations", "duplicate"):
            with self.subTest(fault=fault), self.assertRaises((ValueError, RuntimeError)):
                self.run_capture(fault)
        with self.assertRaises(ValueError):
            measure.capture(Path("unused"), Path("unused"), samples=7)


if __name__ == "__main__":
    unittest.main()
