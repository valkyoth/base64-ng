#!/usr/bin/env python3
"""Negative tests for the dependency forwarding contract."""

import copy
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("companion_features", Path(__file__).with_name("check-companion-features.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class FeatureTests(unittest.TestCase):
    def test_locked_fetch_precedes_all_offline_consumer_checks(self):
        events = []
        with patch.object(module.sys, "argv", ["check", "--companion", "serde"]), \
                patch.object(module.subprocess, "run", side_effect=lambda *a, **k: events.append(("fetch", a, k))), \
                patch.object(module, "check", side_effect=lambda *a, **k: events.append(("check", a, k))):
            module.main()
        active = tomllib.loads((module.ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        self.assertEqual(events[0], ("fetch", (["cargo", f"+{active}", "fetch", "--locked",
                         "--manifest-path", str(module.ROOT / "Cargo.toml")],), {"check": True}))
        self.assertEqual([event[0] for event in events], ["fetch"] + ["check"] * 16)
        self.assertEqual({event[1][0] for event in events[1:]}, {active, "1.90.0"})

    def test_fetch_uses_requested_toolchain_and_failure_stops_matrix(self):
        failure = subprocess.CalledProcessError(101, ["cargo", "fetch"])
        with patch.object(module.sys, "argv", ["check", "--toolchain", "1.90.0"]), \
                patch.object(module.subprocess, "run", side_effect=failure) as fetch, \
                patch.object(module, "check") as check:
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                module.main()
            self.assertIs(raised.exception, failure)
            self.assertEqual(fetch.call_args.args[0][1], "+1.90.0")
            check.assert_not_called()

    def test_audit_only_does_not_fetch_or_run_consumers(self):
        with patch.object(module.sys, "argv", ["check", "--audit-only"]), \
                patch.object(module.subprocess, "run") as fetch, \
                patch.object(module, "check") as check:
            module.main()
            fetch.assert_not_called()
            check.assert_not_called()

    def test_captured_cargo_failure_displays_stderr_and_stays_offline(self):
        failure = subprocess.CalledProcessError(101, ["cargo", "metadata"],
                                              stderr="missing locked dependency\n")
        diagnostic = io.StringIO()
        with patch.object(module.subprocess, "run", side_effect=failure) as run, \
                patch.object(module.sys, "stderr", diagnostic):
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                module.run("1.99.0", Path("/tmp"), "metadata", capture=True)
            self.assertIs(raised.exception, failure)
            self.assertEqual(run.call_args.args[0], ["cargo", "+1.99.0", "metadata", "--offline"])
        self.assertEqual(diagnostic.getvalue(), failure.stderr)

    def test_uncaptured_failure_does_not_duplicate_diagnostics(self):
        diagnostic = io.StringIO()
        with patch.object(module.subprocess, "run", side_effect=subprocess.CalledProcessError(
                101, ["cargo", "test"], stderr="already streamed\n")), \
                patch.object(module.sys, "stderr", diagnostic):
            with self.assertRaises(subprocess.CalledProcessError):
                module.run("1.99.0", Path("/tmp"), "test")
        self.assertEqual(diagnostic.getvalue(), "")

    def test_isolated_consumer_overrides_both_inherited_build_directories(self):
        with patch.dict(os.environ, CARGO_TARGET_DIR="/wrong/target", CARGO_BUILD_BUILD_DIR="/wrong/build"), \
                patch.object(module.subprocess, "run") as run:
            module.run("1.90.0", Path("/tmp"), "check")
            env = run.call_args.kwargs["env"]
            target = module.ROOT / "target/companion-features/1.90.0"
            self.assertEqual(env["CARGO_TARGET_DIR"], str(target))
            self.assertEqual(env["CARGO_BUILD_BUILD_DIR"], str(target / "build"))

    def test_workspace_commands_use_toolchain_specific_build_directories(self):
        with tempfile.TemporaryDirectory(prefix="base64-companion-cache-") as directory:
            work = Path(directory)
            (work / "rust-toolchain.toml").write_bytes((module.ROOT / "rust-toolchain.toml").read_bytes())
            bin_dir = work / "bin"
            bin_dir.mkdir()
            for tool in ("cargo", "python3", "rustup", "rustfmt"):
                stub = bin_dir / tool
                stub.write_text('#!/bin/sh\nif [ "${0##*/}" = cargo ]; then\n'
                                'printf "%s|%s|%s\\n" "$1" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" >> "$CALL_LOG"\n'
                                'if [ "$FAIL_CARGO" = 1 ]; then exit 17; fi\nfi\n')
                stub.chmod(0o700)
            log = work / "calls"
            env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ["PATH"],
                       CALL_LOG=str(log), FAIL_CARGO="0", CARGO_TARGET_DIR="/wrong/target",
                       CARGO_BUILD_BUILD_DIR="/wrong/build")
            command = ["sh", str(module.ROOT / "scripts/check-2.1-companion-features.sh")]
            subprocess.run(command, cwd=work, env=env, check=True, capture_output=True, timeout=20)
            compilers = set()
            for line in log.read_text().splitlines():
                compiler, target, build = line.split("|")
                compilers.add(compiler[1:])
                expected = work / "target/companion-workspace" / compiler[1:]
                self.assertEqual(target, str(expected))
                self.assertEqual(build, str(expected / "build"))
            active = tomllib.loads((work / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
            self.assertEqual(compilers, {active, "1.90.0"})
            env["FAIL_CARGO"] = "1"
            result = subprocess.run(command, cwd=work, env=env, capture_output=True, timeout=20)
            self.assertEqual(result.returncode, 17)

    def test_actual_contract(self):
        module.audit()
        self.assertEqual(module.expected_features("serde", [], False), set())
        self.assertEqual(module.expected_features("bytes", ["simd"], False), {"alloc", "simd"})
        self.assertEqual(module.expected_features("multibase", ["checked-backend"], False), {"simd", "checked-backend"})
        self.assertEqual(module.expected_features("tokio", ["checked-backend"], False), {"std", "alloc", "simd", "checked-backend"})

    def test_missing_forwarding_and_changed_defaults_are_rejected(self):
        original = module.manifest
        for key, value in (("simd", []), ("simd", ["base64-ng/std"]),
                           ("checked-backend", ["simd"]), ("default", ["std", "simd"])):
            def changed(name):
                data = copy.deepcopy(original(name))
                if name == "bytes":
                    data["features"][key] = value
                return data
            with self.subTest(key=key, value=value), patch.object(module, "manifest", changed):
                with self.assertRaises(AssertionError):
                    module.audit()

    def test_implicit_core_defaults_and_secret_only_forwarding_are_rejected(self):
        original = module.manifest
        for mutation in ("defaults", "implicit-std", "secret-forwarding"):
            def changed(name):
                data = copy.deepcopy(original(name))
                if name == "serde" and mutation == "defaults":
                    data["dependencies"]["base64-ng"]["default-features"] = True
                if name == "serde" and mutation == "implicit-std":
                    data["dependencies"]["base64-ng"]["features"] = ["std"]
                if name == "subtle" and mutation == "secret-forwarding":
                    data["features"]["simd"] = ["base64-ng/simd"]
                return data
            with self.subTest(mutation=mutation), patch.object(module, "manifest", changed):
                with self.assertRaises(AssertionError):
                    module.audit()


if __name__ == "__main__":
    unittest.main()
