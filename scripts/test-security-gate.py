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


def run_gate(args, fail="", mutation=None, repeat=1, interrupt="", interrupt_tool="miri"):
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
        for tool in ("miri", "kani"):
            stale = root / f"target/security-2.1/{tool}/stale-artifact"
            stale.parent.mkdir()
            stale.write_text(f"/old-checkout/target/{tool}/runner")
        bin_dir = root / "bin"
        bin_dir.mkdir()
        stub = f'''#!{sys.executable}
import json, os, signal, sys
from pathlib import Path
record = [Path(sys.argv[0]).name, sys.argv[1:], os.environ.get("CARGO_TARGET_DIR"), os.environ.get("CARGO_BUILD_BUILD_DIR"), os.environ.get("CARGO_INCREMENTAL")]
with open(os.environ["TEST_LOG"], "a") as log:
    log.write(json.dumps(record) + "\\n")
verification_tool = next((tool for tool in ("miri", "kani") if tool in sys.argv[1:]), None)
if verification_tool:
    if "CARGO_BUILD_BUILD_DIR" in os.environ or os.environ.get("CARGO_INCREMENTAL") != "0":
        sys.exit(18)
    target = Path(os.environ["CARGO_TARGET_DIR"])
    if not target.name.startswith(verification_tool + ".") or not target.is_dir() or target.stat().st_mode & 0o777 != 0o700:
        sys.exit(19)
    if "--test" in sys.argv[1:] or "ordinary_table_validation_refines_scalar_for_two_quanta" in sys.argv[1:]:
        if list(target.iterdir()):
            sys.exit(19)
        (target / "runner-artifact").write_text(str(target))
    elif not (target / "runner-artifact").is_file():
        sys.exit(19)
    if os.environ["VERIFY_SIGNAL"] and os.environ["INTERRUPT_TOOL"] == verification_tool:
        os.kill(os.getppid(), getattr(signal, os.environ["VERIFY_SIGNAL"]))
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
                   TEST_LOG=str(log), FAIL_MATCH=fail, VERIFY_SIGNAL=interrupt,
                   INTERRUPT_TOOL=interrupt_tool,
                   CARGO_BUILD_BUILD_DIR=str(root / "inherited-build"), CARGO_INCREMENTAL="1")
        for _ in range(repeat):
            result = subprocess.run(["/bin/sh", "gate.sh", *args], cwd=root, env=env,
                                    capture_output=True, text=True, timeout=30)
            for tool in ("miri", "kani"):
                assert not list((root / "target/security-2.1").glob(f"{tool}.*")), f"{tool} target leaked"
                stale = root / f"target/security-2.1/{tool}/stale-artifact"
                assert stale.read_text() == f"/old-checkout/target/{tool}/runner", "Existing cache changed"
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

    def test_verifier_environment_controls_cannot_be_removed(self):
        for tool in ("miri", "kani"):
            block = (f'export CARGO_TARGET_DIR="${tool}_target"\n'
                     'unset CARGO_BUILD_BUILD_DIR\nexport CARGO_INCREMENTAL=0')
            for control in ("unset CARGO_BUILD_BUILD_DIR", "export CARGO_INCREMENTAL=0"):
                result, calls = run_gate(["--extended"], mutation=(block, block.replace(control, ":")))
                self.assertEqual(result.returncode, 18, result.stdout + result.stderr)
                self.assertIn(tool, calls[-1][1])
                self.assertNotIn("smokes passed", result.stdout)

    def test_tool_failure_stops_without_claiming_success(self):
        for args, failed in (([], "test-x86-validation-asm.py"),
                             ([], "--test decode_ref"),
                             (["--extended"], "miri test"),
                             (["--extended"], "v2::ordinary_decode::retained::tests"),
                             (["--extended"], "in_place_bulk_miri_preserved_source_and_scalar_repair"),
                             (["--extended"], "ordinary_table_validation_refines_scalar_for_two_quanta"),
                             (["--extended"], "bulk_in_place_chunk_geometry_preserves_unread_suffix")):
            result, calls = run_gate(args, failed)
            self.assertEqual(result.returncode, 17, result.stdout + result.stderr)
            self.assertIn(failed, " ".join(calls[-1][1]))
            self.assertNotIn("smokes passed", result.stdout)

    def test_verifier_targets_are_fresh_across_runs_in_the_same_checkout(self):
        result, calls = run_gate(["--extended"], "bulk_in_place_chunk_geometry_preserves_unread_suffix", repeat=2)
        self.assertEqual(result.returncode, 17, result.stdout + result.stderr)
        for verifier, count in (("miri", 3), ("kani", 2)):
            targets = [target for tool, args, target, _, _ in calls
                       if tool == "cargo" and verifier in args]
            self.assertEqual(len(targets), count * 2)
            self.assertEqual(len(set(targets[:count])), 1)
            self.assertEqual(len(set(targets[count:])), 1)
            self.assertNotEqual(targets[0], targets[count])

    def test_missing_verifier_cleanup_is_rejected(self):
        for tool in ("miri", "kani"):
            with self.assertRaisesRegex(AssertionError, f"{tool} target leaked"):
                command = "miri test" if tool == "miri" else "kani --no-default-features"
                run_gate(["--extended"], command,
                         mutation=(f'trap \'rm -rf "${tool}_target"\' EXIT', ":"))

    def test_verifier_interruptions_clean_up_and_stop_the_gate(self):
        for tool in ("miri", "kani"):
            for signal, status in (("SIGINT", 130), ("SIGTERM", 143)):
                result, calls = run_gate(["--extended"], interrupt=signal, interrupt_tool=tool)
                self.assertEqual(result.returncode, status, result.stdout + result.stderr)
                self.assertIn(tool, calls[-1][1])
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
            verifier = next((name for name in ("miri", "kani") if name in args), None)
            if verifier:
                self.assertTrue(Path(target).name.startswith(verifier + "."))
                self.assertIsNone(build)
                self.assertEqual(incremental, "0")
            else:
                self.assertEqual(incremental, "1")
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
