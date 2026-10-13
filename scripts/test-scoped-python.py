#!/usr/bin/env python3
"""Exercise isolated scoped launchers against real unchecked-hash bytecode."""

import importlib.util
import json
import os
from pathlib import Path
import py_compile
import select
import signal
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent


class LauncherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="base64-scoped-python-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.scripts = self.root / "scripts"
        self.scripts.mkdir()
        self.cache_parent = self.root / "cache parent"
        self.cache_parent.mkdir()
        self.env = dict(os.environ, TMPDIR=str(self.cache_parent))
        # The selected test interpreter must also execute the launcher so the
        # malicious cache has its actual cache tag, including under python -O.
        bindir = self.root / "bin"
        bindir.mkdir()
        (bindir / "python3").symlink_to(sys.executable)
        self.env["PATH"] = str(bindir) + os.pathsep + os.environ["PATH"]
        self.launcher = self.scripts / "run-scoped-python.sh"
        self.launcher.write_bytes((ROOT / "scripts/run-scoped-python.sh").read_bytes())
        self.marker = self.root / "poison-executed"
        self.module = self.scripts / "scoped_probe.py"
        self.module.write_text("value = 'source'\n")
        evil = self.root / "evil.py"
        evil.write_text(f"from pathlib import Path\nPath({str(self.marker)!r}).touch()\nvalue = 'poison'\n")
        self.poison = Path(importlib.util.cache_from_source(str(self.module), optimization=""))
        py_compile.compile(str(evil), cfile=str(self.poison), doraise=True,
                           invalidation_mode=py_compile.PycInvalidationMode.UNCHECKED_HASH)
        self.poison_bytes = self.poison.read_bytes()
        evil.unlink()
        self.program = (
            "import json, os, stat, sys\nfrom pathlib import Path\nimport scoped_probe\n"
            "cache = Path(sys.pycache_prefix)\n"
            "print(json.dumps({'args': sys.argv[1:], 'value': scoped_probe.value,\n"
            " 'cache': str(cache), 'mode': stat.S_IMODE(cache.stat().st_mode),\n"
            " 'isolated': sys.flags.isolated, 'pid': os.getpid()}), flush=True)\n")
        for name in ("scoped_index.py", "scoped_finalizer.py"):
            (self.scripts / name).write_text(self.program)

    def run_launcher(self, program="scoped_index.py", *args, env=None):
        return subprocess.run(["sh", str(self.launcher), program, *args], cwd=self.root,
                              env=env or self.env, capture_output=True, text=True, timeout=20)

    def check_result(self, result, args):
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(value["value"], "source")
        self.assertEqual(value["args"], args)
        self.assertEqual(value["mode"], 0o700)
        self.assertEqual(value["isolated"], 1)
        self.assertFalse(Path(value["cache"]).exists())
        self.assertEqual(list(self.cache_parent.iterdir()), [])
        self.assertFalse(self.marker.exists())
        self.assertEqual(self.poison.read_bytes(), self.poison_bytes)
        return value

    def test_poison_is_executable_despite_clean_git_but_all_scoped_modes_ignore_it(self):
        (self.root / ".gitignore").write_text("__pycache__/\ncache parent/\nbin/\n")
        for args in (["init", "-q"], ["add", "."],
                     ["-c", "user.name=fixture", "-c", "user.email=test@example.invalid",
                      "-c", "commit.gpgsign=false", "commit", "-qm", "fixture"]):
            subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)
        status = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=all"],
                                         cwd=self.root)
        self.assertEqual(status, b"")
        # Demonstrate that -B does not prevent the reported cache execution.
        control = subprocess.run([sys.executable, "-I", "-B", "-c",
                                  "import sys; sys.path.insert(0, sys.argv[1]); import scoped_probe",
                                  str(self.scripts)], capture_output=True, timeout=10)
        self.assertEqual(control.returncode, 0, control.stderr)
        self.assertTrue(self.marker.exists())
        self.marker.unlink()
        caches = set()
        for program, args in (("scoped_index.py", ["--write"]),
                              ("scoped_index.py", ["--verify"]),
                              ("scoped_finalizer.py", ["--reviews", "path with spaces", "", "$(false)"])):
            with self.subTest(program=program, args=args):
                value = self.check_result(self.run_launcher(program, *args), args)
                self.assertNotIn(value["cache"], caches)
                caches.add(value["cache"])

    def test_shell_finalizer_routes_write_and_review_through_isolation(self):
        for flag, args in (("--scoped-finalize", ["--write"]), ("--scoped-review", [])):
            result = subprocess.run(["sh", str(ROOT / "scripts/finalize-release-evidence.sh"), flag],
                                    cwd=self.root, env=self.env, capture_output=True, text=True, timeout=20)
            self.check_result(result, args)

    def test_hostile_python_environment_and_site_customization_are_ignored(self):
        hostile = self.root / "hostile"
        hostile.mkdir()
        code = f"from pathlib import Path\nPath({str(self.marker)!r}).touch()\n"
        for name in ("sitecustomize.py", "usercustomize.py", "scoped_probe.py"):
            (hostile / name).write_text(code)
        env = dict(self.env, PYTHONPATH=str(hostile), PYTHONHOME=str(hostile),
                   PYTHONUSERBASE=str(hostile), PYTHONPYCACHEPREFIX=str(hostile), PYTHONOPTIMIZE="2")
        self.check_result(self.run_launcher(env=env), [])

    def test_failure_exit_status_and_cleanup(self):
        for suffix in ("raise SystemExit(23)\n", "raise RuntimeError('fixture failure')\n"):
            (self.scripts / "scoped_index.py").write_text(self.program + suffix)
            result = self.run_launcher()
            self.assertEqual(result.returncode, 23 if "SystemExit" in suffix else 1)
            self.assertFalse(Path(json.loads(result.stdout)["cache"]).exists())
            self.assertEqual(list(self.cache_parent.iterdir()), [])

    def test_signals_stop_child_and_remove_cache(self):
        (self.scripts / "scoped_index.py").write_text(self.program + "import time\ntime.sleep(60)\n")
        for sig in (signal.SIGHUP, signal.SIGINT, signal.SIGTERM):
            with self.subTest(signal=sig):
                with subprocess.Popen(["sh", str(self.launcher), "scoped_index.py"], cwd=self.root,
                                      env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                      text=True, start_new_session=True) as process:
                    try:
                        # Wait for a bounded readiness marker, not a blocking readline.
                        ready, _, _ = select.select([process.stdout], [], [], 15)
                        self.assertTrue(ready, "launcher never became ready")
                        value = json.loads(process.stdout.readline())
                        process.send_signal(sig)
                        process.communicate(timeout=10)
                        self.assertEqual(process.returncode, 128 + sig)
                        self.assertFalse(Path(value["cache"]).exists())
                        with self.assertRaises(ProcessLookupError):
                            os.kill(value["pid"], 0)
                    finally:
                        if process.poll() is None:
                            os.killpg(process.pid, signal.SIGKILL)
                            process.communicate(timeout=5)
                self.assertEqual(list(self.cache_parent.iterdir()), [])

    def test_unknown_program_fails_without_creating_cache(self):
        for name in ("", "../scoped_index.py", "other.py"):
            result = self.run_launcher(name)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(list(self.cache_parent.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
