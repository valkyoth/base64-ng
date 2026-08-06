#!/usr/bin/env python3
"""State, isolation, and command-safety tests for the fuzz evidence manager."""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import fuzz_evidence_jobs as jobs  # noqa: E402
import fuzz_evidence_session as session  # noqa: E402


def must_fail(operation: object, description: str) -> None:
    try:
        operation()
    except session.ManagerError:
        return
    raise AssertionError(f"manager accepted {description}")


def main() -> None:
    temporary = Path(tempfile.mkdtemp(prefix="base64-ng-fuzz-manager-test-"))
    collection_parent: Path | None = None
    try:
        state = temporary / "state.sqlite3"
        store = session.Store(state)
        assert state.stat().st_mode & 0o777 == 0o600
        source = session.source_identity(require_clean=False)
        created = store.create_session(source, "https://example.invalid/base64-ng.git")
        collection_parent = created.collection.parent
        assert created.source_commit == source.commit
        assert len(store.jobs()) == 19
        assert [row["target"] for row in store.jobs()] == session.release_targets()
        assert not store.any_running()
        assert not store.all_complete()

        with store.connection:
            store.connection.execute(
                "DELETE FROM jobs WHERE target=?", (session.HARDWARE_TARGET,)
            )
            store.connection.execute(
                """UPDATE jobs SET mode='remote', host='192.0.2.20', port=NULL,
                remote_user='ubuntu' WHERE ordinal=1"""
            )
        store.close()
        store = session.Store(state)
        assert len(store.jobs()) == 19
        assert store.job(session.HARDWARE_TARGET)["status"] == "pending"
        assert store.jobs()[0]["port"] == 22
        store.reset_job(store.jobs()[0]["target"])

        first, second = session.release_targets()[:2]
        store.update(
            first,
            status="running",
            mode="local",
            pid=123,
            started_at=100,
            work_dir=str(temporary / "local"),
        )
        assert store.local_running()
        assert store.any_running()
        store.update(first, status="failed", exit_code=1)
        store.reset_job(first)
        assert store.job(first)["status"] == "pending"

        key = temporary / "worker.pem"
        key.write_text("fixture path only; never loaded by the manager")
        session.validate_remote("ubuntu", "192.0.2.10", 22, key)
        must_fail(
            lambda: session.validate_remote("ubuntu;id", "192.0.2.10", 22, key),
            "shell syntax in a remote user",
        )
        must_fail(
            lambda: session.validate_remote(
                "ubuntu", "-oProxyCommand=id", 22, key
            ),
            "SSH option injection through a host",
        )
        must_fail(
            lambda: session.validate_remote(
                "ubuntu", "192.0.2.10", 22, temporary / "missing"
            ),
            "a missing private-key path",
        )
        must_fail(
            lambda: session.validate_remote("ubuntu", "192.0.2.10", 0, key),
            "an out-of-range SSH port",
        )
        must_fail(
            lambda: session.validate_remote("ubuntu", "192.0.2.10", True, key),
            "a boolean SSH port",
        )
        expected_work = "base64-ng-fuzz-session-decode-attempt"
        assert session.validate_remote_work_dir(
            f"/home/ubuntu/{expected_work}", expected_work
        ).endswith(expected_work)
        must_fail(
            lambda: session.validate_remote_work_dir(
                "/home/ubuntu/worker;touch-pwned", expected_work
            ),
            "shell syntax in a returned remote work directory",
        )
        must_fail(
            lambda: session.validate_remote_work_dir(
                "/home/ubuntu/../other", expected_work
            ),
            "traversal in a returned remote work directory",
        )
        managed_known_hosts = temporary / "known_hosts"
        host_key = temporary / "host-key"
        subprocess.run(
            ["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(host_key)],
            check=True,
        )
        algorithm, public_key, *_ = host_key.with_suffix(".pub").read_text().split()
        managed_known_hosts.write_text(
            f"192.0.2.10 {algorithm} {public_key}\n"
            f"[192.0.2.12]:985 {algorithm} {public_key}\n"
            f"192.0.2.11 {algorithm} {public_key}\n"
        )
        original_known_hosts = session.MANAGED_KNOWN_HOSTS
        session.MANAGED_KNOWN_HOSTS = managed_known_hosts
        try:
            session.reset_managed_known_host("192.0.2.10", 22)
            assert managed_known_hosts.is_file()
            assert managed_known_hosts.stat().st_mode & 0o777 == 0o600
            known_hosts_text = managed_known_hosts.read_text()
            assert "192.0.2.10 " not in known_hosts_text
            assert "192.0.2.11 " in known_hosts_text
            session.reset_managed_known_host("192.0.2.12", 985)
            assert "[192.0.2.12]:985 " not in managed_known_hosts.read_text()
        finally:
            session.MANAGED_KNOWN_HOSTS = original_known_hosts

        store.update(
            second,
            status="running",
            mode="remote",
            host="192.0.2.10",
            port=985,
            remote_user="ubuntu",
            key_path=str(key),
            pid=456,
            started_at=101,
            work_dir="/home/ubuntu/worker",
        )
        assert store.remote_host_running("192.0.2.10", 985)
        assert not store.remote_host_running("192.0.2.10", 22)
        assert not store.remote_host_running("192.0.2.11", 985)
        store.update(second, status="failed", exit_code=2)
        assert not store.remote_host_running("192.0.2.10", 985)

        must_fail(
            lambda: store.update(first, attacker_controlled_column="value"),
            "a non-whitelisted SQLite column",
        )
        must_fail(
            lambda: store.create_session(source, "https://example.invalid/duplicate.git"),
            "a second session in one database",
        )
        symlink = temporary / "state-link.sqlite3"
        symlink.symlink_to(state)
        must_fail(lambda: session.Store(symlink), "a symlinked manager database")

        runner = temporary / "runner.sh"
        output = temporary / "collection"
        session.write_local_runner(runner, "decode", output, "safe-label")
        runner_text = runner.read_text()
        assert str(ROOT) in runner_text
        assert "capture-fuzz-shard.sh decode" in runner_text
        assert str(output) in runner_text
        assert "cleanup_lock" in runner_text
        assert runner.stat().st_mode & 0o777 == 0o700

        hardware_runner = temporary / "hardware-runner.sh"
        session.write_local_runner(
            hardware_runner, session.HARDWARE_TARGET, output, "safe-label"
        )
        hardware_runner_text = hardware_runner.read_text()
        assert "capture-2.0-riscv-admission.sh" in hardware_runner_text
        assert str(session.hardware_bundle(output)) in hardware_runner_text
        assert "capture-fuzz-shard.sh" not in hardware_runner_text

        assert f"cargo-fuzz {session.FUZZ_VERSION}" in jobs.REMOTE_BOOTSTRAP
        assert 'if [ "$target" = "riscv_hardware" ]' in jobs.REMOTE_BOOTSTRAP
        assert "capture-2.0-riscv-admission.sh" in jobs.REMOTE_BOOTSTRAP
        assert 'rustup run "$project_toolchain" rustc -V' in jobs.REMOTE_BOOTSTRAP
        assert 'export RUSTUP_TOOLCHAIN="$project_toolchain"' in jobs.REMOTE_BOOTSTRAP
        assert 'scripts/ci_install_rust.sh' not in jobs.REMOTE_BOOTSTRAP
        assert "base64-ng-fuzz-$session-$target-$attempt" in jobs.REMOTE_BOOTSTRAP
        assert "missing_commands" in jobs.REMOTE_BOOTSTRAP
        assert "for required in cc git curl python3" in jobs.REMOTE_BOOTSTRAP
        assert "build-essential pkg-config" in jobs.REMOTE_BOOTSTRAP
        assert "pkgconf-pkg-config" in jobs.REMOTE_BOOTSTRAP
        assert "run_root zypper --non-interactive" in jobs.REMOTE_BOOTSTRAP
        assert "run_root apk add build-base" in jobs.REMOTE_BOOTSTRAP
        assert jobs.REMOTE_BOOTSTRAP.index('export PATH="$HOME/.cargo/bin:$PATH"') < (
            jobs.REMOTE_BOOTSTRAP.index("command -v rustup")
        )
        assert jobs.REMOTE_BOOTSTRAP.count('missing="$(missing_commands)"') == 2
        subprocess.run(
            ["bash", "-n"], input=jobs.REMOTE_BOOTSTRAP, text=True, check=True
        )
        subprocess.run(["bash", "-n"], input=jobs.REMOTE_STATUS, text=True, check=True)
        subprocess.run(["sh", "-n", str(runner)], check=True)
        assert "StrictHostKeyChecking=accept-new" in session.ssh_command(
            "ubuntu", "192.0.2.10", 985, key
        )
        assert "985" in session.ssh_command("ubuntu", "192.0.2.10", 985, key)
        assert f"UserKnownHostsFile={session.MANAGED_KNOWN_HOSTS}" in session.ssh_command(
            "ubuntu", "192.0.2.10", 985, key
        )
        assert "GlobalKnownHostsFile=/dev/null" in session.scp_command(
            "ubuntu", "192.0.2.10", 985, key
        )
        assert "985" in session.scp_command("ubuntu", "192.0.2.10", 985, key)
        assert key.read_text() not in " ".join(
            session.ssh_command("ubuntu", "192.0.2.10", 985, key)
        )

        for target in session.release_targets():
            store.update(target, status="complete", finished_at=int(time.time()), exit_code=0)
        assert store.all_complete()
        store.close()

        reopened = session.Store(state)
        assert reopened.session() == created
        assert reopened.all_complete()
        reopened.close()
        status = subprocess.run(
            [
                "python3",
                str(ROOT / "scripts" / "manage-fuzz-evidence.py"),
                "--state",
                str(state),
                "--status",
            ],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        assert f"source_commit={source.commit}" in status
        assert status.count("=complete") == 19
    finally:
        if collection_parent is not None:
            shutil.rmtree(collection_parent, ignore_errors=True)
        shutil.rmtree(temporary, ignore_errors=True)
    print("fuzz evidence manager tests: ok")


if __name__ == "__main__":
    main()
