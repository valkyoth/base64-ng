"""Check artifact selection without assuming Cargo's private directory layout."""
import json
from pathlib import Path
import runpy
import tempfile
import unittest

executable = runpy.run_path(str(Path(__file__).with_name("cargo-test-executable.py")))["executable"]


class ArtifactTests(unittest.TestCase):
    def test_exactly_one_local_library_test(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build = root / "target"
            build.mkdir()
            binary = build / "arbitrary-name"
            binary.touch(mode=0o700)
            outside = root / "unrelated"
            outside.touch(mode=0o700)
            messages = root / "messages.jsonl"
            entry = dict(reason="compiler-artifact", target=dict(name="base64_ng", kind=["lib"]),
                         profile=dict(test=True), executable=str(binary))
            for entries, valid in (
                ([{"reason": "build-finished"}, entry], True),
                ([], False), ([entry, entry], False),
                ([dict(entry, profile=dict(test=False))], False),
                ([dict(entry, executable=str(outside))], False),
            ):
                with self.subTest(entries=entries):
                    messages.write_text("".join(json.dumps(row) + "\n" for row in entries))
                    if valid:
                        self.assertEqual(executable(messages, build), binary.resolve())
                    else:
                        with self.assertRaises(ValueError):
                            executable(messages, build)


if __name__ == "__main__":
    unittest.main()
