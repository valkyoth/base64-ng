#!/usr/bin/env python3
"""Exercise the native performance capture command without native hardware."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
CAPTURE = ROOT / "scripts/capture-2.0-riscv-admission.sh"


class CaptureTests(unittest.TestCase):
    def capture(self, source):
        preamble = source.split('output_dir=', 1)[0]
        block = source.split('echo "RVV admission capture: production-detection', 1)[1]
        block = 'echo "RVV admission capture: production-detection' + block.split(
            '\nif [ "$source_commit"', 1
        )[0]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            cargo = root / "cargo"
            cargo.write_text(
                '#!/usr/bin/env python3\n'
                'import json, os, sys\n'
                'print(json.dumps(dict(args=sys.argv[1:], env=dict(os.environ))))\n'
            )
            cargo.chmod(0o700)
            validator = root / "scripts/validate-rvv-performance.py"
            validator.write_text('#!/bin/sh\nexit 0\n')
            validator.chmod(0o700)
            env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}",
                       CARGO_ENCODED_RUSTFLAGS='--cfg\x1fbase64_ng_rvv_candidate',
                       RUSTFLAGS='--cfg base64_ng_rvv_candidate -C target-feature=+v',
                       temporary=str(root), samples='15', target_bytes='4194304')
            subprocess.run(['sh', '-eu', '-c', preamble + block], cwd=root, env=env,
                           check=True, capture_output=True, text=True)
            result = json.loads((root / 'rvv.csv').read_text())
        self.assertNotIn('CARGO_ENCODED_RUSTFLAGS', result['env'])
        self.assertEqual(result['env']['RUSTFLAGS'], '--cfg base64_ng_perf_evidence')
        self.assertEqual(result['env']['BASE64_NG_PERF_SAMPLES'], '15')
        self.assertEqual(result['env']['BASE64_NG_PERF_TARGET_BYTES'], '4194304')
        self.assertEqual(result['args'], ['run', '--locked', '--quiet', '--release',
                                         '--manifest-path', 'perf/Cargo.toml', '--', 'rvv'])

    def test_inherited_flags_cannot_select_candidate_or_global_vector_isa(self):
        self.capture(CAPTURE.read_text())

    def test_mutations_are_detected(self):
        source = CAPTURE.read_text()
        for old, new in (
            ('unset CARGO_ENCODED_RUSTFLAGS', ':'),
            ("RUSTFLAGS='--cfg base64_ng_perf_evidence'",
             "RUSTFLAGS='--cfg base64_ng_perf_evidence --cfg base64_ng_rvv_candidate'"),
            ('cargo run --locked', 'cargo run'),
        ):
            with self.subTest(mutation=old), self.assertRaises(AssertionError):
                self.capture(source.replace(old, new))


if __name__ == '__main__':
    unittest.main()
