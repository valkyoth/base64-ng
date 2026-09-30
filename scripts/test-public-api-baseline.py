#!/usr/bin/env python3
"""Mutation tests for the paired benchmark schema, statistics, and case matrix."""

import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

from public_api_baseline import classify, parse_sample, summarize

spec = importlib.util.spec_from_file_location("runner", Path(__file__).with_name("compare-2.1-public-api.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class BaselineTests(unittest.TestCase):
    def test_overlay_removes_stale_revision_harness_files(self):
        with tempfile.TemporaryDirectory() as directory:
            tree = Path(directory)
            stale = tree / "perf/public-api/build.rs"
            stale.parent.mkdir(parents=True)
            stale.write_text("stale build script")
            (tree / "perf/src").mkdir()
            (tree / "src/v2").mkdir(parents=True)
            source = runner.ROOT / "perf/public-api/Cargo.toml"
            runner.install_harness(tree, [source])
            self.assertFalse(stale.exists())
            self.assertEqual((tree / "perf/public-api/Cargo.toml").read_bytes(), source.read_bytes())

    def test_measurement_schema_rejects_missing_false_and_invalid_accounting(self):
        valid = dict(elapsed_ns="100", iterations="2", raw_bytes="3", encoded_bytes="4",
                     allocations="0", encode_capability="scalar", decode_capability="scalar")
        self.assertEqual(parse_sample(valid, 2, 3, 4)["elapsed_ns"], 100)
        for field, value in [("elapsed_ns", "0"), ("iterations", "0"), ("iterations", "3"),
                             ("raw_bytes", "4"), ("encoded_bytes", "5"), ("allocations", "-1"),
                             ("elapsed_ns", "nan")]:
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                parse_sample(dict(valid, **{field: value}), 2, 3, 4)
        del valid["allocations"]
        with self.assertRaises(ValueError):
            parse_sample(valid, 2, 3, 4)

    def test_noisy_short_and_consistent_comparisons(self):
        self.assertEqual(classify([100] * 7, [100] * 7)[1], "within-5-percent")
        self.assertEqual(classify([100] * 7, [80] * 7)[1], "improvement-signal")
        self.assertEqual(classify([100] * 7, [120] * 7)[1], "regression-signal")
        self.assertEqual(classify([100], [80])[1], "insufficient-samples")
        self.assertEqual(classify([100] * 7, [120] * 4 + [100] * 3)[1], "inconclusive")
        self.assertEqual(classify([100] * 7, [80] * 4 + [100] * 3)[1], "inconclusive")
        self.assertEqual(classify([100] * 7, [50, 70, 90, 100, 110, 130, 150])[1], "noisy")
        for a, b in [([], []), ([1], [1, 2]), ([float("nan")], [1]), ([0], [1])]:
            with self.assertRaises(ValueError):
                classify(a, b)

    def test_rejection_is_latency_only_and_missing_pairs_fail(self):
        rows = [dict(features="default", case=["canonical", "decode", "sp", "3", "random", "1", "invalid-0"],
                     side=side, sample=0, elapsed_ns=100, iterations=1, allocations=0,
                     raw_bytes=3, encoded_bytes=4) for side in ["baseline", "candidate"]]
        self.assertIsNone(summarize(rows)[0]["payload_gib_s"])
        with self.assertRaises(ValueError):
            summarize(rows[:1])
        with self.assertRaises(ValueError):
            summarize(rows + rows)
        bad = copy.deepcopy(rows)
        bad[1]["sample"] = 1
        with self.assertRaises(ValueError):
            summarize(bad)

    def test_full_matrix_covers_boundaries_profiles_patterns_and_fragments(self):
        matrix = list(runner.cases(["canonical", "validate", "incremental", "tokio", "bytes", "sync"], False, True))
        self.assertEqual(len(matrix), len({tuple(row) for row in matrix}))
        self.assertTrue({"sp", "su", "up", "uu"}.issubset({row[2] for row in matrix}))
        self.assertTrue({"0", "1", "2", "3", "32", "512", "4096", "65536", "1048576"}.issubset({row[3] for row in matrix}))
        self.assertEqual({row[4] for row in matrix}, {"random", "zero", "structured"})
        self.assertEqual({row[5] for row in matrix}, {"1", "7", "4096"})
        for row in matrix:
            if row[0] == "validate":
                self.assertEqual(row[1], "decode")
            if row[-1].startswith("invalid-"):
                length = (int(row[3]) + 2) // 3 * 4 if row[2].endswith("p") else (int(row[3]) * 8 + 5) // 6
                self.assertLess(int(row[-1].split("-")[1]), length)


if __name__ == "__main__":
    unittest.main()
