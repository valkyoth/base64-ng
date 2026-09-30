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

from public_api_sandbox import BUILD_ENV, HOST_ENV, RUNTIME_ENV, MEMORY_LIMITS, Sandbox, bounded
from public_api_cgroup import verify
import public_api_sandbox as isolation

spec = importlib.util.spec_from_file_location("runner", Path(__file__).with_name("compare-2.1-public-api.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def process_stopped(pid):
    try:
        state = Path(f"/proc/{pid}/stat").read_text().split()[2]
    except (FileNotFoundError, ProcessLookupError):
        # procfs can lose the process either before open or during the read.
        return True
    return state == "Z"


class OutputTests(unittest.TestCase):
    def test_process_exit_observation_handles_procfs_races(self):
        for error in (FileNotFoundError(), ProcessLookupError()):
            with self.subTest(error=type(error).__name__), \
                    patch.object(Path, 'read_text', side_effect=error):
                self.assertTrue(process_stopped(123))
        for state in ('R', 'S', 'D', 'T', 'Z'):
            with self.subTest(state=state), \
                    patch.object(Path, 'read_text', return_value=f'123 (child) {state} 1'):
                self.assertEqual(process_stopped(123), state == 'Z')
        with patch.object(Path, 'read_text', side_effect=PermissionError()):
            with self.assertRaises(PermissionError):
                process_stopped(123)

    def test_provenance_tracks_policy_and_all_launchers(self):
        expected = {Path('/usr/bin') / name for name in
                    ('bwrap', 'prlimit', 'systemd-run', 'systemctl', 'python3')}
        self.assertEqual(set(isolation.SECURITY_TOOLS), expected)
        self.assertTrue(expected <= set(runner.tool_inventory(Path('/toolchain'))))
        with patch.dict(MEMORY_LIMITS, {True: 4096, False: 2048}), \
                patch.object(isolation, 'SWAP_LIMIT', 16), \
                patch.object(isolation, 'TASK_LIMIT', 32), \
                patch.object(isolation, 'CPU_QUOTA_PERCENT', 100):
            self.assertEqual(runner.aggregate_limits(), dict(build_memory_bytes=4096,
                             runtime_memory_bytes=2048, swap_bytes=16, tasks=32, cpu_quota_percent=100))
            sandbox = object.__new__(Sandbox)
            with patch.object(sandbox, 'command', return_value=['/usr/bin/true']), \
                    patch.object(isolation, 'bounded', return_value=b'') as launch:
                sandbox.execute(['/usr/bin/true'])
            command = launch.call_args_list[0].args[0]
            for prop in ['MemoryMax=2048', 'MemorySwapMax=16', 'TasksMax=32', 'CPUQuota=100%']:
                self.assertIn('--property=' + prop, command)
            self.assertEqual(command[-5:], ['2048', '16', '32', '100', '/usr/bin/true'])

    def test_cgroup_limits_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            membership = root / 'membership'
            membership.write_text('0::/scope\n')
            group = root / 'scope'
            group.mkdir()
            values = {'memory.max': '1073741824', 'memory.swap.max': '0',
                      'pids.max': '128', 'cpu.max': '200000 100000'}
            for name, value in values.items():
                (group / name).write_text(value)
            verify(1073741824, 0, 128, 200, root, membership)
            for name, value in values.items():
                with self.subTest(name=name):
                    (group / name).write_text('max 100000' if name == 'cpu.max' else 'max')
                    with self.assertRaises(RuntimeError):
                        verify(1073741824, 0, 128, 200, root, membership)
                    (group / name).unlink()
                    with self.assertRaises(FileNotFoundError):
                        verify(1073741824, 0, 128, 200, root, membership)
                    (group / name).write_text(value)

            for name, value in {'memory.max': '4096', 'memory.swap.max': '16',
                                'pids.max': '32', 'cpu.max': '50000 100000'}.items():
                (group / name).write_text(value)
            verify(4096, 16, 32, 50, root, membership)

    def test_archive_member_limit_precedes_extraction(self):
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode='w') as archive:
            for i in range(10001):
                archive.addfile(tarfile.TarInfo(f'empty-{i}'))
        with tempfile.TemporaryDirectory() as directory, patch.object(runner, 'bounded', return_value=buffer.getvalue()):
            with self.assertRaisesRegex(ValueError, 'member count'):
                runner.extract_revision('ignored', Path(directory))
            self.assertEqual(list(Path(directory).iterdir()), [])

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
                if process_stopped(pid):
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

    def test_forked_allocations_share_one_memory_budget(self):
        # Each child fits the per-process limit; together they exceed the cgroup.
        code = """
import subprocess,sys
children = [subprocess.Popen(['/usr/bin/python3', '-c',
    'import time; data=bytearray(32*1024*1024); time.sleep(2)']) for _ in range(3)]
codes = [child.wait() for child in children]
sys.exit(1 if any(codes) else 0)
"""
        with patch.dict(MEMORY_LIMITS, {False: 64 * 1024**2}):
            with self.assertRaises(RuntimeError):
                self.sandbox.execute(['/usr/bin/python3', '-c', code], timeout=10)
        self.assertEqual(self.sandbox.execute(['/usr/bin/true']), b'')

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
