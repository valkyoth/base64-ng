#!/usr/bin/env python3
"""Local and remote execution operations for the fuzz evidence manager."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

from fuzz_evidence_session import (
    FUZZ_SECONDS,
    FUZZ_VERSION,
    HARDWARE_TARGET,
    ROOT,
    ManagerError,
    Store,
    acquire_local_lock,
    pid_alive,
    reset_managed_known_host,
    scp_command,
    ssh_command,
    validate_remote,
    validate_remote_work_dir,
    hardware_bundle,
    write_local_runner,
)


REMOTE_BOOTSTRAP = r'''set -eu
target="$1"
commit="$2"
session="$3"
repository="$4"
bootstrap_rustup="$5"
install_prerequisites="$6"
attempt="$7"
case "$target$session$commit$attempt" in *[!A-Za-z0-9._-]*) exit 64;; esac
export PATH="$HOME/.cargo/bin:$PATH"

missing_commands() {
    missing=""
    for required in cc git curl python3 tar gzip awk sed grep find wc; do
        if ! command -v "$required" >/dev/null 2>&1; then
            missing="${missing}${missing:+ }$required"
        fi
    done
    printf '%s\n' "$missing"
}

run_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        command -v sudo >/dev/null 2>&1 || {
            echo "remote worker: sudo is required to install system prerequisites" >&2
            exit 69
        }
        sudo -n "$@"
    fi
}

missing="$(missing_commands)"
if [ -n "$missing" ]; then
    if [ "$install_prerequisites" != "yes" ]; then
        echo "remote worker: missing system prerequisites: $missing" >&2
        echo "remote worker: approve prerequisite installation or provision the host manually" >&2
        exit 69
    fi
    if command -v apt-get >/dev/null 2>&1; then
        run_root apt-get update
        run_root env DEBIAN_FRONTEND=noninteractive apt-get install -y \
            build-essential pkg-config git curl python3 tar gzip ca-certificates \
            gawk sed grep findutils coreutils
    elif command -v dnf >/dev/null 2>&1; then
        run_root dnf install -y gcc gcc-c++ make pkgconf-pkg-config git curl \
            python3 tar gzip ca-certificates gawk sed grep findutils coreutils
    elif command -v yum >/dev/null 2>&1; then
        run_root yum install -y gcc gcc-c++ make pkgconfig git curl python3 tar \
            gzip ca-certificates gawk sed grep findutils coreutils
    elif command -v zypper >/dev/null 2>&1; then
        run_root zypper --non-interactive install gcc gcc-c++ make pkg-config \
            git curl python3 tar gzip ca-certificates gawk sed grep findutils coreutils
    elif command -v apk >/dev/null 2>&1; then
        run_root apk add build-base pkgconf git curl python3 tar gzip \
            ca-certificates gawk sed grep findutils coreutils bash
    else
        echo "remote worker: no supported system package manager was found" >&2
        exit 69
    fi
fi
missing="$(missing_commands)"
if [ -n "$missing" ]; then
    echo "remote worker: prerequisites remain missing after installation: $missing" >&2
    exit 69
fi
if ! command -v rustup >/dev/null 2>&1; then
    if [ "$bootstrap_rustup" != "yes" ]; then
        echo "remote worker: rustup is missing and bootstrap was not approved" >&2
        exit 69
    fi
    installer="$(mktemp)"
    trap 'rm -f "$installer"' EXIT
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$installer"
    sh "$installer" -y --profile minimal
    rm -f "$installer"
    trap - EXIT
fi

work="$HOME/base64-ng-fuzz-$session-$target-$attempt"
if [ -e "$work" ]; then
    echo "remote worker: refusing existing work directory: $work" >&2
    exit 73
fi
git clone --filter=blob:none --no-checkout "$repository" "$work"
cd "$work"
git checkout --detach "$commit"
test "$(git rev-parse HEAD)" = "$commit"
test -z "$(git status --porcelain --untracked-files=all)"
project_toolchain="$(
    sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml \
        | sed -n '1p'
)"
if [ -z "$project_toolchain" ]; then
    echo "remote worker: rust-toolchain.toml is missing a channel" >&2
    exit 65
fi
if rustup run "$project_toolchain" rustc -V >/dev/null 2>&1 \
    && rustup run "$project_toolchain" cargo -V >/dev/null 2>&1; then
    echo "remote worker: reusing installed Rust $project_toolchain"
else
    echo "remote worker: installing minimal Rust $project_toolchain"
    rustup set profile minimal
    rustup toolchain install "$project_toolchain" --profile minimal
fi
export RUSTUP_TOOLCHAIN="$project_toolchain"
rustc -V
cargo -V
mkdir -p target/fuzz-manager
runner="target/fuzz-manager/run.sh"
status_file="target/fuzz-manager/exit-status"
if [ "$target" = "''' + HARDWARE_TARGET + r'''" ]; then
    cat > "$runner" <<'RUNNER'
#!/usr/bin/env sh
set +e
scripts/capture-2.0-riscv-admission.sh target/riscv-native-admission
status=$?
printf '%s\n' "$status" > "$3.tmp"
mv "$3.tmp" "$3"
exit "$status"
RUNNER
    chmod 700 "$runner"
    label="remote-$session-$target"
    nohup "$runner" "$label" "$target" "$status_file" \
        > target/fuzz-manager/job.log 2>&1 < /dev/null &
    pid=$!
    printf 'MANAGER_PID=%s\nREMOTE_DIR=%s\n' "$pid" "$work"
    exit 0
fi
rustup toolchain install nightly --profile minimal
installed_fuzz="$(cargo +nightly fuzz --version 2>/dev/null || true)"
if [ "$installed_fuzz" != "cargo-fuzz ''' + FUZZ_VERSION + r'''" ]; then
    cargo +nightly install --locked --force cargo-fuzz --version ''' + FUZZ_VERSION + r'''
fi
cat > "$runner" <<'RUNNER'
#!/usr/bin/env sh
set +e
BASE64_NG_FUZZ_MACHINE_LABEL="$1" \
    scripts/capture-fuzz-shard.sh "$2" target/fuzz-shards ''' + str(FUZZ_SECONDS) + r'''
status=$?
printf '%s\n' "$status" > "$3.tmp"
mv "$3.tmp" "$3"
exit "$status"
RUNNER
chmod 700 "$runner"
label="remote-$session-$target"
nohup "$runner" "$label" "$target" "$status_file" \
    > target/fuzz-manager/job.log 2>&1 < /dev/null &
pid=$!
printf 'MANAGER_PID=%s\nREMOTE_DIR=%s\n' "$pid" "$work"
'''


REMOTE_STATUS = r'''set -eu
work="$1"
pid="$2"
status="$work/target/fuzz-manager/exit-status"
if [ -s "$status" ]; then
    printf 'MANAGER_STATE=finished\nMANAGER_EXIT='
    cat "$status"
elif kill -0 "$pid" 2>/dev/null; then
    printf 'MANAGER_STATE=running\n'
else
    printf 'MANAGER_STATE=unknown\n'
fi
'''


def stream_command(command: list[str], script: str) -> str:
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    assert process.stdin is not None and process.stdout is not None
    process.stdin.write(script)
    process.stdin.close()
    lines: list[str] = []
    for line in process.stdout:
        print(line, end="")
        lines.append(line)
    status = process.wait()
    if status != 0:
        raise ManagerError(f"remote setup failed with exit status {status}")
    return "".join(lines)


class JobController:
    def __init__(self, store: Store) -> None:
        self.store = store
        self.session = store.session()
        self.session.collection.mkdir(parents=True, exist_ok=True)
        self.logs = self.session.collection.parent / "manager-logs"
        self.logs.mkdir(parents=True, exist_ok=True)

    def validate_source(self) -> None:
        from fuzz_evidence_session import source_identity

        source = source_identity()
        if (
            source.commit != self.session.source_commit
            or source.tree != self.session.source_tree
        ):
            raise ManagerError("the current source no longer matches this evidence session")

    def validate_bundle(self, target: str) -> None:
        if target == HARDWARE_TARGET:
            subprocess.run(
                [
                    "python3",
                    "scripts/validate-rvv-admission-bundle.py",
                    str(hardware_bundle(self.session.collection)),
                ],
                cwd=ROOT,
                check=True,
            )
            return
        subprocess.run(
            [
                "python3",
                "scripts/fuzz_shard_evidence.py",
                "validate",
                str(self.session.collection / target),
            ],
            cwd=ROOT,
            check=True,
        )

    def start_local(self, target: str) -> None:
        self.validate_source()
        if self.store.local_running():
            raise ManagerError("only one local fuzz target may run at a time")
        row = self.store.job(target)
        if row["status"] != "pending":
            raise ManagerError(f"{target} is not pending")
        attempt = f"{int(time.time())}-{os.getpid()}"
        directory = self.logs / f"local-{target}-{attempt}"
        directory.mkdir()
        runner = directory / "run.sh"
        status_file = directory / "exit-status"
        log_path = directory / "job.log"
        write_local_runner(
            runner, target, self.session.collection, f"local-{self.session.identifier}-{target}"
        )
        lock = acquire_local_lock()
        try:
            with log_path.open("wb") as log:
                process = subprocess.Popen(
                    [str(runner), str(status_file), str(lock)],
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )
        except BaseException:
            lock.rmdir()
            raise
        self.store.update(
            target,
            status="running",
            mode="local",
            work_dir=str(directory),
            pid=process.pid,
            started_at=int(time.time()),
            message=f"local log: {log_path}",
        )

    def start_remote(
        self,
        target: str,
        user: str,
        host: str,
        port: int,
        key_path: Path,
        bootstrap_rustup: bool,
        install_prerequisites: bool,
    ) -> None:
        self.validate_source()
        validate_remote(user, host, port, key_path)
        if self.store.remote_host_running(host, port):
            raise ManagerError(
                f"remote host {host}:{port} already has a running fuzz target"
            )
        row = self.store.job(target)
        if row["status"] != "pending":
            raise ManagerError(f"{target} is not pending")
        attempt = f"{int(time.time())}-{os.getpid()}"
        reset_managed_known_host(host, port)
        command = ssh_command(user, host, port, key_path) + [
            "bash",
            "-s",
            "--",
            target,
            self.session.source_commit,
            self.session.identifier,
            self.session.repository,
            "yes" if bootstrap_rustup else "no",
            "yes" if install_prerequisites else "no",
            attempt,
        ]
        output = stream_command(command, REMOTE_BOOTSTRAP)
        values: dict[str, str] = {}
        for line in output.splitlines():
            if line.startswith(("MANAGER_PID=", "REMOTE_DIR=")):
                key, value = line.split("=", 1)
                values[key] = value
        if set(values) != {"MANAGER_PID", "REMOTE_DIR"} or not values[
            "MANAGER_PID"
        ].isdigit():
            raise ManagerError("remote setup did not return a valid job identity")
        expected = f"base64-ng-fuzz-{self.session.identifier}-{target}-{attempt}"
        work_dir = validate_remote_work_dir(values["REMOTE_DIR"], expected)
        if Path(work_dir).name != expected:
            raise ManagerError("remote setup returned a mismatched work directory")
        self.store.update(
            target,
            status="running",
            mode="remote",
            host=host,
            port=port,
            remote_user=user,
            key_path=str(key_path),
            work_dir=work_dir,
            pid=int(values["MANAGER_PID"]),
            started_at=int(time.time()),
            message=f"remote worker: {user}@{host}:{port}",
        )

    def check(self, target: str) -> str:
        self.validate_source()
        row = self.store.job(target)
        if row["status"] != "running":
            return row["status"]
        if row["mode"] == "local":
            return self._check_local(row)
        return self._check_remote(row)

    def _finish(self, target: str, exit_code: int, message: str) -> str:
        if exit_code == 0:
            try:
                self.validate_bundle(target)
            except subprocess.CalledProcessError as error:
                self.store.update(target, status="failed", message="bundle validation failed")
                raise ManagerError("completed job produced invalid evidence") from error
            status = "complete"
        else:
            status = "failed"
        self.store.update(
            target,
            status=status,
            finished_at=int(time.time()),
            exit_code=exit_code,
            message=message,
        )
        return status

    def _check_local(self, row: object) -> str:
        directory = Path(row["work_dir"])
        status_file = directory / "exit-status"
        if status_file.is_file():
            text = status_file.read_text().strip()
            if not text.isdigit():
                self.store.update(row["target"], status="unknown", message="invalid status file")
                return "unknown"
            return self._finish(
                row["target"], int(text), f"local log: {directory / 'job.log'}"
            )
        if pid_alive(row["pid"]):
            return "running"
        self.store.update(row["target"], status="unknown", message="local process disappeared")
        return "unknown"

    def _remote_query(self, row: object) -> dict[str, str]:
        key = Path(row["key_path"])
        work_dir = self._work_dir(row)
        command = ssh_command(
            row["remote_user"], row["host"], row["port"], key
        ) + [
            "bash",
            "-s",
            "--",
            work_dir,
            str(row["pid"]),
        ]
        result = subprocess.run(
            command,
            input=REMOTE_STATUS,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            raise ManagerError(
                f"could not query {row['remote_user']}@{row['host']}: {result.stderr.strip()}"
            )
        values: dict[str, str] = {}
        for line in result.stdout.splitlines():
            if line.startswith("MANAGER_") and "=" in line:
                key_name, value = line.split("=", 1)
                values[key_name] = value
        return values

    def _work_dir(self, row: object) -> str:
        prefix = f"base64-ng-fuzz-{self.session.identifier}-{row['target']}-"
        return validate_remote_work_dir(row["work_dir"], prefix)

    def _download(self, row: object) -> None:
        target = row["target"]
        is_hardware = target == HARDWARE_TARGET
        destination = (
            hardware_bundle(self.session.collection)
            if is_hardware
            else self.session.collection / target
        )
        if destination.exists():
            self.validate_bundle(target)
            return
        key = Path(row["key_path"])
        work_dir = self._work_dir(row)
        with tempfile.TemporaryDirectory(
            prefix=f".{target}-download-", dir=self.session.collection.parent
        ) as temporary:
            remote_artifact = (
                f"{work_dir}/target/riscv-native-admission"
                if is_hardware
                else f"{work_dir}/target/fuzz-shards/{target}"
            )
            command = scp_command(
                row["remote_user"], row["host"], row["port"], key
            ) + [
                "-r",
                f"{row['remote_user']}@{row['host']}:{remote_artifact}",
                temporary,
            ]
            subprocess.run(command, check=True)
            downloaded = Path(temporary) / (
                "riscv-native-admission" if is_hardware else target
            )
            if not downloaded.is_dir():
                raise ManagerError("remote evidence download did not produce the target bundle")
            shutil.move(str(downloaded), destination)
        try:
            self.validate_bundle(target)
        except BaseException:
            shutil.rmtree(destination, ignore_errors=True)
            raise

    def _download_log(self, row: object) -> Path:
        destination = self.logs / f"remote-{row['target']}.log"
        key = Path(row["key_path"])
        work_dir = self._work_dir(row)
        command = scp_command(
            row["remote_user"], row["host"], row["port"], key
        ) + [
            f"{row['remote_user']}@{row['host']}:{work_dir}/target/fuzz-manager/job.log",
            str(destination),
        ]
        subprocess.run(command, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return destination

    def _check_remote(self, row: object) -> str:
        values = self._remote_query(row)
        state = values.get("MANAGER_STATE")
        if state == "running":
            return "running"
        if state != "finished" or not values.get("MANAGER_EXIT", "").isdigit():
            self.store.update(row["target"], status="unknown", message="remote process disappeared")
            return "unknown"
        exit_code = int(values["MANAGER_EXIT"])
        if exit_code == 0:
            self._download(row)
            message = (
                "verified; remote worker may be terminated: "
                f"{row['host']}:{row['port']}"
            )
        else:
            message = f"remote failure log: {self._download_log(row)}"
        return self._finish(row["target"], exit_code, message)

    def finalize(self) -> None:
        self.validate_source()
        if not self.store.all_complete():
            raise ManagerError("all 18 fuzz targets and native RISC-V evidence must be complete")
        subprocess.run(
            ["scripts/aggregate-fuzz-shards.sh", str(self.session.collection)],
            cwd=ROOT,
            check=True,
        )
