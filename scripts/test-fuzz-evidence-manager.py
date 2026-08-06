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
        assert len(store.jobs()) == 18
        assert [row["target"] for row in store.jobs()] == session.release_targets()
        assert not store.any_running()
        assert not store.all_complete()

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
        session.validate_remote("ubuntu", "192.0.2.10", key)
        must_fail(
            lambda: session.validate_remote("ubuntu;id", "192.0.2.10", key),
            "shell syntax in a remote user",
        )
        must_fail(
            lambda: session.validate_remote("ubuntu", "-oProxyCommand=id", key),
            "SSH option injection through a host",
        )
        must_fail(
            lambda: session.validate_remote("ubuntu", "192.0.2.10", temporary / "missing"),
            "a missing private-key path",
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

        store.update(
            second,
            status="running",
            mode="remote",
            host="192.0.2.10",
            remote_user="ubuntu",
            key_path=str(key),
            pid=456,
            started_at=101,
            work_dir="/home/ubuntu/worker",
        )
        assert store.remote_host_running("192.0.2.10")
        assert not store.remote_host_running("192.0.2.11")
        store.update(second, status="failed", exit_code=2)
        assert not store.remote_host_running("192.0.2.10")

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

        assert f"cargo-fuzz {session.FUZZ_VERSION}" in jobs.REMOTE_BOOTSTRAP
        assert "scripts/ci_install_rust.sh" in jobs.REMOTE_BOOTSTRAP
        assert "base64-ng-fuzz-$session-$target-$attempt" in jobs.REMOTE_BOOTSTRAP
        subprocess.run(
            ["bash", "-n"], input=jobs.REMOTE_BOOTSTRAP, text=True, check=True
        )
        subprocess.run(["bash", "-n"], input=jobs.REMOTE_STATUS, text=True, check=True)
        subprocess.run(["sh", "-n", str(runner)], check=True)
        assert "StrictHostKeyChecking=accept-new" in session.ssh_command(
            "ubuntu", "192.0.2.10", key
        )
        assert key.read_text() not in " ".join(
            session.ssh_command("ubuntu", "192.0.2.10", key)
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
        assert status.count("=complete") == 18
    finally:
        if collection_parent is not None:
            shutil.rmtree(collection_parent, ignore_errors=True)
        shutil.rmtree(temporary, ignore_errors=True)
    print("fuzz evidence manager tests: ok")


if __name__ == "__main__":
    main()
