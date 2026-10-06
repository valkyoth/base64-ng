#!/usr/bin/env python3
"""Exercise security gate routing/failure behavior without running verification tools."""
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def run_gate(args, fail="", mutation=None, repeat=1, interrupt=""):
    with tempfile.TemporaryDirectory(prefix="base64-security-gate-") as directory:
        root = Path(directory)
        source = (ROOT / "scripts/check-2.1-security.sh").read_text()
        if mutation is not None:
            before, after = mutation
            assert before in source
            source = source.replace(before, after, 1)
        (root / "gate.sh").write_text(source)
        (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.99.0"\n')
        (root / "target/security-2.1").mkdir(parents=True)
        stale = root / "target/security-2.1/miri/stale-artifact"
        stale.parent.mkdir()
        stale.write_text("/old-checkout/target/miri/runner")
        bin_dir = root / "bin"
        bin_dir.mkdir()
        stub = f'''#!{sys.executable}
import json, os, signal, sys
from pathlib import Path
record = [Path(sys.argv[0]).name, sys.argv[1:], os.environ.get("CARGO_TARGET_DIR"), os.environ.get("CARGO_BUILD_BUILD_DIR"), os.environ.get("CARGO_INCREMENTAL")]
with open(os.environ["TEST_LOG"], "a") as log:
    log.write(json.dumps(record) + "\\n")
if "miri" in sys.argv[1:]:
    if "CARGO_BUILD_BUILD_DIR" in os.environ or os.environ.get("CARGO_INCREMENTAL") != "0":
        sys.exit(18)
    target = Path(os.environ["CARGO_TARGET_DIR"])
    if not target.is_dir() or target.stat().st_mode & 0o777 != 0o700:
        sys.exit(19)
    if "--test" in sys.argv[1:]:
        if list(target.iterdir()):
            sys.exit(19)
        (target / "runner-artifact").write_text(str(target))
    elif not (target / "runner-artifact").is_file():
        sys.exit(19)
    if os.environ["MIRI_SIGNAL"]:
        os.kill(os.getppid(), getattr(signal, os.environ["MIRI_SIGNAL"]))
        sys.exit(0)
if os.environ["FAIL_MATCH"] and os.environ["FAIL_MATCH"] in " ".join(sys.argv[1:]):
    sys.exit(17)
'''
        for tool in ("cargo", "python3", "sh"):
            path = bin_dir / tool
            path.write_text(stub)
            path.chmod(0o700)
        log = root / "calls.jsonl"
        env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ["PATH"],
                   TEST_LOG=str(log), FAIL_MATCH=fail, MIRI_SIGNAL=interrupt,
                   CARGO_BUILD_BUILD_DIR=str(root / "inherited-build"), CARGO_INCREMENTAL="1")
        for _ in range(repeat):
            result = subprocess.run(["/bin/sh", "gate.sh", *args], cwd=root, env=env,
                                    capture_output=True, text=True, timeout=30)
            assert not list((root / "target/security-2.1").glob("miri.*")), "Miri target leaked"
            assert stale.read_text() == "/old-checkout/target/miri/runner", "Existing cache changed"
        calls = [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
        return result, calls


class GateTests(unittest.TestCase):
    def test_default_never_runs_extended_tools_and_isolates_compilers(self):
        result, calls = run_gate([])
        self.assertEqual(result.returncode, 0, result.stderr)
        cargo = [call for call in calls if call[0] == "cargo"]
        self.assertTrue(cargo)
        self.assertEqual({call[1][0] for call in cargo}, {"+1.99.0", "+1.90.0"})
        for _, args, target, build, incremental in cargo:
            self.assertNotIn("miri", args)
            self.assertNotIn("kani", args)
            self.assertNotIn("fuzz", args)
            self.assertTrue(target.endswith("/" + args[0][1:]))
            self.assertEqual(build, target + "/build")
            self.assertEqual(incremental, "1")

    def test_miri_environment_controls_cannot_be_removed(self):
        for mutation in (("unset CARGO_BUILD_BUILD_DIR", ":"),
                         ("export CARGO_INCREMENTAL=0", ":")):
            result, calls = run_gate(["--extended"], mutation=mutation)
            self.assertEqual(result.returncode, 18, result.stdout + result.stderr)
            self.assertIn("miri", calls[-1][1])
            self.assertNotIn("smokes passed", result.stdout)

    def test_tool_failure_stops_without_claiming_success(self):
        for args, failed in (([], "test-x86-validation-asm.py"),
                             ([], "--test decode_ref"),
                             (["--extended"], "miri test"),
                             (["--extended"], "v2::ordinary_decode::retained::tests"),
                             (["--extended"], "in_place_bulk_miri_preserved_source_and_scalar_repair"),
                             (["--extended"], "kani --no-default-features")):
            result, calls = run_gate(args, failed)
            self.assertEqual(result.returncode, 17, result.stdout + result.stderr)
            self.assertIn(failed, " ".join(calls[-1][1]))
            self.assertNotIn("smokes passed", result.stdout)

    def test_miri_targets_are_fresh_across_runs_in_the_same_checkout(self):
        result, calls = run_gate(["--extended"], "kani --no-default-features", repeat=2)
        self.assertEqual(result.returncode, 17, result.stdout + result.stderr)
        targets = [target for tool, args, target, _, _ in calls
                   if tool == "cargo" and "miri" in args]
        self.assertEqual(len(targets), 6)
        self.assertEqual(len(set(targets[:3])), 1)
        self.assertEqual(len(set(targets[3:])), 1)
        self.assertNotEqual(targets[0], targets[3])

    def test_missing_miri_cleanup_is_rejected(self):
        with self.assertRaisesRegex(AssertionError, "Miri target leaked"):
            run_gate(["--extended"], "miri test",
                     mutation=('trap \'rm -rf "$miri_target"\' EXIT', ":"))

    def test_miri_interruptions_clean_up_and_stop_the_gate(self):
        for signal, status in (("SIGINT", 130), ("SIGTERM", 143)):
            result, calls = run_gate(["--extended"], interrupt=signal)
            self.assertEqual(result.returncode, status, result.stdout + result.stderr)
            self.assertIn("miri", calls[-1][1])
            self.assertNotIn("smokes passed", result.stdout)

    def test_bad_arguments_fail_before_tools(self):
        for args in (["--unknown"], ["--extended", "extra"]):
            result, calls = run_gate(args)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(calls)

    @unittest.skipUnless(platform.system() == "Linux" and platform.machine() == "x86_64",
                         "extended gate intentionally requires native x86_64 Linux")
    def test_extended_requests_bounded_proofs_sanitizers_and_fuzz(self):
        result, calls = run_gate(["--extended"])
        self.assertEqual(result.returncode, 0, result.stderr)
        cargo = [args for tool, args, _, _, _ in calls if tool == "cargo"]
        for tool, args, target, build, incremental in calls:
            if tool != "cargo":
                continue
            if "miri" in args:
                self.assertTrue(Path(target).name.startswith("miri."))
                self.assertIsNone(build)
                self.assertEqual(incremental, "0")
            else:
                self.assertEqual(incremental, "1")
                if "kani" in args:
                    self.assertIsNone(build)
                else:
                    self.assertEqual(build, target + "/build")
        self.assertEqual(sum("miri" in args for args in cargo), 3)
        proofs = [args for args in cargo if "kani" in args]
        self.assertEqual(len(proofs), 2)
        for args in proofs:
            self.assertEqual(args[args.index("--harness-timeout") + 1], "5m")
        self.assertEqual(sum("-Zbuild-std" in args for args in cargo), 1)
        fuzz = [args for args in cargo if "fuzz" in args]
        self.assertEqual(len(fuzz), 2)
        for args in fuzz:
            self.assertIn("-runs=1000", args)
            self.assertIn("-rss_limit_mb=2048", args)
            self.assertIn("-max_len=8192", args)


if __name__ == "__main__":
    unittest.main()
