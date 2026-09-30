"""Linux-only, fail-closed execution boundary for benchmark candidate code."""

import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time
import uuid

HOST_ENV = {"PATH": "/usr/bin:/bin", "HOME": "/nonexistent", "LC_ALL": "C",
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null"}
BUILD_ENV = {"PATH": "/toolchain/bin:/usr/bin:/bin", "HOME": "/work/home",
             "CARGO_HOME": "/work/cargo", "RUSTC": "/toolchain/bin/rustc",
             "RUSTFLAGS": "--cfg base64_ng_perf_evidence", "LC_ALL": "C",
             "CARGO_BUILD_JOBS": "2", "CARGO_INCREMENTAL": "0", "TMPDIR": "/tmp"}
RUNTIME_ENV = {"PATH": "/usr/bin:/bin", "HOME": "/work/home", "LC_ALL": "C", "TMPDIR": "/tmp"}
MEMORY_LIMITS = {True: 6 * 1024**3, False: 1024**3}


def bounded(command, *, env=None, timeout=600, limit=65536, stderr_limit=1048576, cwd=None):
    """Drain both pipes incrementally; kill the process group on every exit path."""
    process = subprocess.Popen(command, env=HOST_ENV if env is None else env, cwd=cwd,
                               stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, start_new_session=True)
    data = {"stdout": bytearray(), "stderr": bytearray()}
    deadline = time.monotonic() + timeout
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ, ("stdout", limit))
            selector.register(process.stderr, selectors.EVENT_READ, ("stderr", stderr_limit))
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("child exceeded wall-clock limit")
                for key, _ in selector.select(min(remaining, 0.1)):
                    name, ceiling = key.data
                    chunk = os.read(key.fd, min(65536, ceiling - len(data[name]) + 1))
                    if not chunk:
                        selector.unregister(key.fileobj)
                    elif len(data[name]) + len(chunk) > ceiling:
                        raise ValueError(f"child {name} exceeded {ceiling} bytes")
                    else:
                        data[name].extend(chunk)
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("child exceeded wall-clock limit")
            code = process.wait(timeout=remaining)
            if code:
                raise RuntimeError(f"child exited {code}: {data['stderr'].decode('utf-8', errors='replace')}")
        return bytes(data["stdout"])
    finally:
        # bwrap's PID namespace also kills descendants that create new sessions.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        process.stdout.close()
        process.stderr.close()


class Sandbox:
    def __init__(self, toolchain):
        if sys.platform != "linux":
            raise RuntimeError("paired candidate execution requires Linux isolation; no unsandboxed fallback")
        import pwd

        if os.getuid() == 0:
            raise RuntimeError("run the benchmark sandbox as an unprivileged user")
        for path in ("/usr/bin/bwrap", "/usr/bin/prlimit", "/usr/bin/cc",
                     "/usr/bin/systemd-run", "/usr/bin/systemctl", "/usr/bin/python3"):
            if not Path(path).is_file():
                raise RuntimeError(f"sandbox prerequisite missing: {path}; no unsandboxed fallback")
        home = Path(pwd.getpwuid(os.getuid()).pw_dir)
        rustup = Path("/usr/bin/rustup") if Path("/usr/bin/rustup").is_file() else home / ".cargo/bin/rustup"
        discovery_env = dict(HOST_ENV, HOME=str(home))
        compiler = bounded([str(rustup), "which", "--toolchain", toolchain, "rustc"], env=discovery_env).decode().strip()
        self.toolchain = Path(compiler).resolve().parent.parent
        self.registry = home / ".cargo/registry"
        if not self.registry.is_dir():
            raise RuntimeError("prefetch the locked registry dependencies before offline capture")
        # Probe namespace availability before extracting/building candidate code.
        self.execute(["/usr/bin/true"])

    def command(self, command, *, tree=None, binary=None, build=False):
        args = ["/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--disable-userns", "--die-with-parent",
                "--new-session", "--cap-drop", "ALL", "--clearenv", "--ro-bind", "/usr", "/usr"]
        for path in ("/bin", "/lib", "/lib64"):
            if Path(path).is_symlink():
                args += ["--symlink", os.readlink(path), path]
            elif Path(path).exists():
                args += ["--ro-bind", path, path]
        args += ["--ro-bind-try", "/etc/ld.so.cache", "/etc/ld.so.cache", "--proc", "/proc", "--dev", "/dev",
                 "--size", "67108864", "--tmpfs", "/tmp", "--size", "1073741824" if build else "67108864",
                 "--tmpfs", "/work", "--dir", "/work/home", "--chdir", "/work"]
        # Debian-family compiler symlinks traverse /etc/alternatives.
        if Path('/etc/alternatives').is_dir():
            args += ['--ro-bind', '/etc/alternatives', '/etc/alternatives']
        if build:
            args += ["--ro-bind", str(self.toolchain), "/toolchain", "--dir", "/work/cargo",
                     "--ro-bind", str(self.registry), "/work/cargo/registry"]
        if tree is not None:
            args += ["--ro-bind", str(tree), "/source"]
        if binary is not None:
            args += ["--ro-bind", str(binary), "/benchmark"]
        # Do not leave the namespace root or device tmpfs as unbounded scratch.
        args += ["--remount-ro", "/", "--remount-ro", "/dev"]
        for name, value in (BUILD_ENV if build else RUNTIME_ENV).items():
            args += ["--setenv", name, value]
        return [*args, "/usr/bin/prlimit", "--core=0", "--cpu=600", "--nproc=128", "--nofile=256",
                f"--as={4294967296 if build else 536870912}", "--fsize=67108864", "--", *command]

    def execute(self, command, *, tree=None, binary=None, build=False, limit=65536, timeout=600):
        unit = f"base64-ng-benchmark-{uuid.uuid4().hex}.scope"
        memory = MEMORY_LIMITS[build]
        env = dict(HOST_ENV, XDG_RUNTIME_DIR=f"/run/user/{os.getuid()}",
                   DBUS_SESSION_BUS_ADDRESS=f"unix:path=/run/user/{os.getuid()}/bus")
        args = ["/usr/bin/systemd-run", "--user", "--scope", "--quiet", "--collect",
                f"--unit={unit}", f"--property=MemoryMax={memory}",
                "--property=MemorySwapMax=0", "--property=TasksMax=128",
                "--property=CPUQuota=200%", "/usr/bin/python3", "-I",
                str(Path(__file__).with_name('public_api_cgroup.py')), str(memory),
                *self.command(command, tree=tree, binary=binary, build=build)]
        try:
            return bounded(args, env=env, limit=limit,
                           stderr_limit=4 * 1024 * 1024 if build else 65536, timeout=timeout)
        finally:
            # A timeout must also dispose of processes moved to the transient scope.
            # Already collected units return nonzero; no candidate is launched here.
            try:
                bounded(["/usr/bin/systemctl", "--user", "stop", unit], env=env, timeout=15)
            except RuntimeError:
                pass

    def compile(self, tree, features):
        # The entire target directory lives on a quota-limited disposable tmpfs.
        # Export only the bounded executable; no candidate can write host files.
        command = ["/bin/sh", "-c", '"$@" >&2 && cat /work/target/release/base64-ng-public-api-perf', "build",
                   "/toolchain/bin/cargo", "build", "--release", "--locked", "--offline", "--no-default-features",
                   "--manifest-path", "/source/perf/public-api/Cargo.toml", "--target-dir", "/work/target"]
        if features:
            command += ["--features", features]
        return self.execute(command, tree=tree, build=True, limit=64 * 1024 * 1024)
