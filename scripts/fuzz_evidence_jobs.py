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
    ROOT,
    ManagerError,
    Store,
    acquire_local_lock,
    pid_alive,
    scp_command,
    ssh_command,
    validate_remote,
    validate_remote_work_dir,
    write_local_runner,
)


REMOTE_BOOTSTRAP = r'''set -eu
target="$1"
commit="$2"
session="$3"
repository="$4"
bootstrap_rustup="$5"
attempt="$6"
case "$target$session$commit$attempt" in *[!A-Za-z0-9._-]*) exit 64;; esac
for required in git curl python3 tar awk sed grep find wc; do
    command -v "$required" >/dev/null 2>&1 || {
        echo "remote worker: missing required command: $required" >&2
        exit 69
    }
done
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
export PATH="$HOME/.cargo/bin:$PATH"
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
scripts/ci_install_rust.sh
rustup toolchain install nightly --profile minimal
installed_fuzz="$(cargo +nightly fuzz --version 2>/dev/null || true)"
if [ "$installed_fuzz" != "cargo-fuzz ''' + FUZZ_VERSION + r'''" ]; then
    cargo +nightly install --locked --force cargo-fuzz --version ''' + FUZZ_VERSION + r'''
fi
mkdir -p target/fuzz-manager
runner="target/fuzz-manager/run.sh"
status_file="target/fuzz-manager/exit-status"
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
        key_path: Path,
        bootstrap_rustup: bool,
    ) -> None:
        self.validate_source()
        validate_remote(user, host, key_path)
        if self.store.remote_host_running(host):
            raise ManagerError(f"remote host {host} already has a running fuzz target")
        row = self.store.job(target)
        if row["status"] != "pending":
            raise ManagerError(f"{target} is not pending")
        attempt = f"{int(time.time())}-{os.getpid()}"
        command = ssh_command(user, host, key_path) + [
            "bash",
            "-s",
            "--",
            target,
            self.session.source_commit,
            self.session.identifier,
            self.session.repository,
            "yes" if bootstrap_rustup else "no",
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
            remote_user=user,
            key_path=str(key_path),
            work_dir=work_dir,
            pid=int(values["MANAGER_PID"]),
            started_at=int(time.time()),
            message=f"remote worker: {user}@{host}",
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
        command = ssh_command(row["remote_user"], row["host"], key) + [
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
        destination = self.session.collection / target
        if destination.exists():
            self.validate_bundle(target)
            return
        key = Path(row["key_path"])
        work_dir = self._work_dir(row)
        with tempfile.TemporaryDirectory(
            prefix=f".{target}-download-", dir=self.session.collection.parent
        ) as temporary:
            command = scp_command(row["remote_user"], row["host"], key) + [
                "-r",
                f"{row['remote_user']}@{row['host']}:{work_dir}/target/fuzz-shards/{target}",
                temporary,
            ]
            subprocess.run(command, check=True)
            downloaded = Path(temporary) / target
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
        command = scp_command(row["remote_user"], row["host"], key) + [
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
            message = f"verified; remote worker may be terminated: {row['host']}"
        else:
            message = f"remote failure log: {self._download_log(row)}"
        return self._finish(row["target"], exit_code, message)

    def finalize(self) -> None:
        self.validate_source()
        if not self.store.all_complete():
            raise ManagerError("all 18 targets must be complete before final verification")
        subprocess.run(
            ["scripts/aggregate-fuzz-shards.sh", str(self.session.collection)],
            cwd=ROOT,
            check=True,
        )
