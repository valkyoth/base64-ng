#!/usr/bin/env python3
"""Regression tests for candidate isolation and bounded child communication."""

import importlib.util
import io
import os
from pathlib import Path
import signal
import socket
import sys
import tarfile
import tempfile
import time
import tomllib
import unittest
from unittest.mock import patch

from public_api_sandbox import BUILD_ENV, HOST_ENV, RUNTIME_ENV, Sandbox, bounded

spec = importlib.util.spec_from_file_location("runner", Path(__file__).with_name("compare-2.1-public-api.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class OutputTests(unittest.TestCase):
    def test_output_limits_are_enforced_during_reading(self):
        for fd, expected in [(1, "stdout"), (2, "stderr")]:
            with self.subTest(fd=fd), self.assertRaisesRegex(ValueError, expected):
                bounded([sys.executable, "-c", f"import os\nwhile True: os.write({fd}, b'x'*4096)"],
                        limit=8192, stderr_limit=8192, timeout=3)
        self.assertEqual(bounded([sys.executable, "-c", "print('ok')"]), b"ok\n")

    def test_timeout_kills_child_and_group(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "pid"
            code = "import subprocess,time,pathlib,sys; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); pathlib.Path(sys.argv[1]).write_text(str(p.pid)); time.sleep(60)"
            with self.assertRaises(TimeoutError):
                bounded([sys.executable, "-c", code, str(marker)], timeout=0.5)
            pid = int(marker.read_text())
            for _ in range(100):
                stat = Path(f"/proc/{pid}/stat")
                if not stat.exists() or stat.read_text().split()[2] == "Z":
                    break
                time.sleep(0.01)
            else:
                os.kill(pid, signal.SIGKILL)
                self.fail("descendant survived group termination")

    def test_archive_links_and_traversal_are_rejected(self):
        for name, kind in [("../escape", tarfile.REGTYPE), ("link", tarfile.SYMTYPE)]:
            buffer = io.BytesIO()
            with tarfile.open(fileobj=buffer, mode="w") as archive:
                member = tarfile.TarInfo(name)
                member.type = kind
                member.linkname = "/tmp"
                archive.addfile(member)
            with tempfile.TemporaryDirectory() as directory, patch.object(runner, "bounded", return_value=buffer.getvalue()):
                with self.assertRaisesRegex(ValueError, "unsafe path/type"):
                    runner.extract_revision("ignored", Path(directory))


class IsolationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Required, never silently skipped: CI must provision bwrap/user namespaces.
        toolchain = tomllib.loads((runner.ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        cls.sandbox = Sandbox(toolchain)

    def test_actual_cargo_build_script_and_exported_binary_are_confined(self):
        with tempfile.TemporaryDirectory() as directory:
            tree = Path(directory)
            package = tree / "perf/public-api"
            (package / "src").mkdir(parents=True)
            (tree / "canary").write_text("unchanged")
            (package / "Cargo.toml").write_text('[package]\nname="base64-ng-public-api-perf"\nversion="0.0.0"\nedition="2024"\n[workspace]\n')
            (package / "Cargo.lock").write_text('version=4\n[[package]]\nname="base64-ng-public-api-perf"\nversion="0.0.0"\n')
            (package / "build.rs").write_text('''fn main() {
    assert!(std::env::var("AWS_SECRET_ACCESS_KEY").is_err());
    assert!(std::env::var("RUSTC_WRAPPER").is_err());
    assert!(std::fs::write("/source/canary", "changed").is_err());
    assert!(!std::path::Path::new("/home").exists());
}''')
            (package / "src/main.rs").write_text('fn main() { println!("sandbox-pass"); }')
            with patch.dict(os.environ, RUSTC_WRAPPER="/evil/wrapper", AWS_SECRET_ACCESS_KEY="test-sentinel"):
                binary = tree / "exported-binary"
                binary.write_bytes(self.sandbox.compile(tree, ""))
                binary.chmod(0o700)
                self.assertEqual(self.sandbox.execute(["/benchmark"], binary=binary), b"sandbox-pass\n")
            self.assertEqual((tree / "canary").read_text(), "unchanged")

    def test_environment_and_network_and_source_are_isolated(self):
        with tempfile.TemporaryDirectory() as directory, socket.socket() as listener:
            tree = Path(directory)
            (tree / "canary").write_text("public source")
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            code = '''
import os,pathlib,socket,sys
if os.environ.get('SSH_AUTH_SOCK') or os.environ.get('RUSTC_WRAPPER') or os.environ.get('AWS_SECRET_ACCESS_KEY'):
    sys.exit('environment leaked')
if pathlib.Path(sys.argv[1]).exists(): sys.exit('host path visible')
try: pathlib.Path('/source/canary').write_text('changed')
except OSError: pass
else: sys.exit('source writable')
for path in ['/unbounded-file', '/dev/unbounded-file', '/dev/shm/unbounded-file']:
    try: pathlib.Path(path).write_text('changed')
    except OSError: pass
    else: sys.exit('unbounded scratch mount')
try: socket.create_connection(('127.0.0.1', int(sys.argv[2])), timeout=0.2)
except OSError: pass
else: sys.exit('host network reachable')
print('isolated')
'''
            with patch.dict(os.environ, SSH_AUTH_SOCK="/host/socket", RUSTC_WRAPPER="/evil/wrapper",
                            AWS_SECRET_ACCESS_KEY="test-sentinel", LD_PRELOAD="/evil/preload.so",
                            CARGO_HOME="/evil", CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="evil"):
                for build in [False, True]:
                    result = self.sandbox.execute(["/usr/bin/python3", "-c", code, str(tree / "canary"), str(listener.getsockname()[1])], tree=tree, build=build)
                    self.assertEqual(result, b"isolated\n")
            self.assertEqual((tree / "canary").read_text(), "public source")

    def test_tmpfs_quota_and_environment_are_explicit(self):
        command = self.sandbox.command(["/usr/bin/true"], build=True)
        self.assertIn("--unshare-all", command)
        self.assertIn("--clearenv", command)
        self.assertNotIn("--bind", command)
        for env in [BUILD_ENV, RUNTIME_ENV, HOST_ENV]:
            self.assertNotIn("LD_PRELOAD", env)
            self.assertNotIn("SSH_AUTH_SOCK", env)
        code = '''
import pathlib,sys
try:
    for i in range(2):
        with pathlib.Path('/work/file'+str(i)).open('wb') as f:
            for _ in range(40): f.write(b'x' * 1048576)
except OSError: print('quota')
else: sys.exit('tmpfs was not bounded')
'''
        self.assertEqual(self.sandbox.execute(["/usr/bin/python3", "-c", code]), b"quota\n")
        self.assertEqual(self.sandbox.execute(["/usr/bin/ls", "-A", "/work"]), b"home\n")

    def test_detached_descendant_does_not_keep_namespace_alive(self):
        code = "import subprocess; subprocess.Popen(['/usr/bin/sleep','60'],start_new_session=True); print('done')"
        start = time.monotonic()
        self.assertEqual(self.sandbox.execute(["/usr/bin/python3", "-c", code], timeout=3), b"done\n")
        self.assertLess(time.monotonic() - start, 3)


if __name__ == "__main__":
    unittest.main()
