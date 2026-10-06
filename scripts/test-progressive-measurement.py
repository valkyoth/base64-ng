#!/usr/bin/env python3
"""Reject incomplete or malformed paired progressive decision measurements."""
import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("progressive_measurement", Path(__file__).with_name("measure-2.1-progressive.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def fixture():
    lines = []
    for profile in range(4):
        for size in module.SIZES:
            lines.append(f"progressive-backend,{profile},{size},avx2")
            for sample in range(11):
                for mode, nanos in enumerate((1200, 1000, 1500)):
                    rounds = max(4, 4194304 // size)
                    lines.append(f"progressive-bench,{profile},{size},{sample},{mode},{rounds},{nanos}")
    return "\n".join(lines)


class MeasurementTests(unittest.TestCase):
    def test_redaction_precedence_repeated_paths_and_idempotence(self):
        with patch.object(module, "ROOT", Path("/home/reviewer/project")), \
                patch.object(module.Path, "home", return_value=Path("/home/reviewer")):
            text = "build /home/reviewer/project\n/home/reviewer/.cargo/bin\n/home/reviewer/project/src/lib.rs\n"
            expected = "build <REPOSITORY>\n<HOME>/.cargo/bin\n<REPOSITORY>/src/lib.rs\n"
            self.assertEqual(module.redact_build_log(text), expected)
            self.assertEqual(module.redact_build_log(expected), expected)
            self.assertEqual(module.redact_build_log(fixture()), fixture())

    def test_saved_log_and_digest_use_redacted_bytes_without_changing_samples(self):
        with tempfile.TemporaryDirectory(prefix="base64-redaction-") as directory, \
                patch.object(module, "ROOT", Path("/work/checkout")), \
                patch.object(module.Path, "home", return_value=Path("/home/reviewer")):
            text = "Compiling (/work/checkout)\n/home/reviewer/.rustup\n" + fixture() + "\n\n"
            path = Path(directory) / "capture.log"
            digest = module.write_log(path, text)
            stored = path.read_bytes()
            self.assertEqual(digest, hashlib.sha256(stored).hexdigest())
            self.assertNotEqual(digest, hashlib.sha256(text.encode()).hexdigest())
            self.assertNotIn(b"/work/checkout", stored)
            self.assertNotIn(b"/home/reviewer", stored)
            self.assertTrue(stored.endswith(b"\n\n"))
            self.assertEqual(module.parse(stored.decode()), module.parse(text))

    def test_retained_capture_matches_raw_logs_and_complete_repetitions(self):
        directory = module.ROOT / "docs/evidence/progressive-2.1"
        record = json.loads((directory / "summary.json").read_text())
        self.assertEqual(record["schema"], 1)
        self.assertEqual(len(record["runs"]), 6)
        self.assertEqual({(r["features"], r["repetition"]) for r in record["runs"]},
                         {(f, r) for f in ("std,simd", "std,checked-backend") for r in range(3)})
        for run in record["runs"]:
            name = f"{'checked' if 'checked' in run['features'] else 'plain'}-{run['repetition']}.log"
            self.assertEqual(run["log"], name)
            log = (directory / name).read_bytes()
            self.assertNotRegex(log.decode(), r"/(?:home|Users)/")
            self.assertEqual(hashlib.sha256(log).hexdigest(), run["log_sha256"])
            self.assertEqual(module.parse(log.decode()), run["rows"])

    def test_complete_pairs_and_medians(self):
        rows = module.parse(fixture())
        self.assertEqual(len(rows), 16)
        for row in rows:
            self.assertEqual(row["transactional_over_progressive"], [1.2] * 11)
            self.assertEqual(row["median_speedup"], 1.2)

    def test_missing_duplicate_and_unknown_samples_rejected(self):
        text = fixture()
        for changed in ("\n".join(text.splitlines()[1:]),
                        "\n".join(text.splitlines()[:-1]),
                        text + "\n" + text.splitlines()[1],
                        text.replace("progressive-bench,0,4096,0,0", "progressive-bench,0,4096,0,3")):
            with self.assertRaises(ValueError):
                module.parse(changed)

    def test_wrong_rounds_timings_and_backends_rejected(self):
        text = fixture()
        for old, new in ((",1024,1200", ",1023,1200"), (",1024,1200", ",1024,0"),
                         (",1024,1200", ",1024,-1"), (",1024,1200", ",1024,nan"),
                         (",avx2", ",scalar")):
            with self.subTest(new=new), self.assertRaises(ValueError):
                module.parse(text.replace(old, new))


if __name__ == "__main__":
    unittest.main()
