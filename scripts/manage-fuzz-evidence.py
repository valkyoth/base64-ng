#!/usr/bin/env python3
"""Interactive, resumable manager for distributed release evidence."""

from __future__ import annotations

import argparse
import os
import sqlite3
import subprocess
import sys
from datetime import datetime
from pathlib import Path

from fuzz_evidence_jobs import JobController
from fuzz_evidence_session import (
    DEFAULT_STATE,
    ManagerError,
    Store,
    source_identity,
)


DEFAULT_REPOSITORY = "https://github.com/valkyoth/base64-ng.git"


def answer(prompt: str, default: str | None = None) -> str:
    suffix = f" [{default}]" if default else ""
    value = input(f"{prompt}{suffix}: ").strip()
    return value or (default or "")


def confirm(prompt: str, default: bool = False) -> bool:
    suffix = " [Y/n]" if default else " [y/N]"
    value = input(f"{prompt}{suffix}: ").strip().lower()
    if not value:
        return default
    return value in {"y", "yes"}


def remove_database(path: Path) -> None:
    for suffix in ("", "-wal", "-shm"):
        candidate = Path(f"{path}{suffix}")
        if candidate.exists():
            candidate.unlink()


def create_store(path: Path, repository: str) -> Store:
    source = source_identity()
    store = Store(path)
    session = store.create_session(source, repository)
    print(f"Created session {session.identifier}")
    print(f"Pinned commit: {session.source_commit}")
    print(f"Evidence collection: {session.collection}")
    return store


def open_interactive(path: Path, force_new: bool, repository: str) -> Store:
    exists = path.exists()
    if exists and not force_new:
        print("A previous fuzz evidence session exists.")
        print("1. Continue previous session")
        print("2. Start new session")
        print("q. Quit")
        choice = answer("Selection")
        if choice == "1":
            return Store(path)
        if choice != "2":
            raise KeyboardInterrupt
    if exists:
        existing = Store(path)
        try:
            if existing.any_running():
                raise ManagerError(
                    "cannot reset state while jobs are marked running; "
                    "continue and check them first"
                )
        finally:
            existing.close()
        if not force_new and not confirm(
            "Reset the SQLite state? Existing evidence directories are preserved"
        ):
            raise KeyboardInterrupt
        remove_database(path)
    return create_store(path, repository)


def elapsed(started_at: int | None) -> str:
    if started_at is None:
        return ""
    seconds = max(0, int(datetime.now().timestamp()) - started_at)
    hours, remainder = divmod(seconds, 3600)
    minutes = remainder // 60
    return f" {hours:02d}:{minutes:02d}"


def status_text(row: sqlite3.Row) -> str:
    status = row["status"]
    if status == "pending":
        return "pending"
    if status == "running":
        location = (
            "local"
            if row["mode"] == "local"
            else f"{row['host']}:{row['port']}"
        )
        return f"CHECK PROGRESS ({location}){elapsed(row['started_at'])}"
    if status == "complete":
        return "COMPLETE"
    return f"{status.upper()}: {row['message']}"


def show_menu(store: Store) -> None:
    session = store.session()
    jobs = store.jobs()
    counts = {status: 0 for status in ("pending", "running", "complete", "failed", "unknown")}
    for row in jobs:
        counts[row["status"]] += 1
    print("\nbase64-ng distributed release evidence")
    print(f"Session: {session.identifier}")
    print(f"Commit:  {session.source_commit}")
    print(
        "Status:  "
        + " ".join(f"{name}={count}" for name, count in counts.items() if count)
    )
    print()
    for row in jobs:
        print(f"{row['ordinal']:2d}. {row['target']:<22} {status_text(row)}")
    print(" c. Check every running job")
    if store.all_complete():
        print(" f. FINAL VERIFICATION AND AGGREGATION")
    print(" q. Save state and quit")


def remote_defaults(store: Store) -> tuple[str, str, str]:
    previous = store.last_remote()
    default_user = os.environ.get("BASE64_NG_FUZZ_SSH_USER", "ubuntu")
    default_key = os.environ.get("BASE64_NG_FUZZ_SSH_KEY", "")
    default_port = os.environ.get("BASE64_NG_FUZZ_SSH_PORT", "22")
    if previous is not None:
        default_user = previous["remote_user"] or default_user
        default_key = previous["key_path"] or default_key
        default_port = str(previous["port"] or default_port)
    return default_user, default_key, default_port


def start_pending(store: Store, controller: JobController, target: str) -> None:
    print(f"\nStart {target}")
    print("1. Run local")
    print("2. Run remote over SSH")
    print("b. Back")
    choice = answer("Selection")
    if choice == "1":
        controller.start_local(target)
        print(f"{target} started locally; it will continue if this manager exits.")
    elif choice == "2":
        default_user, default_key, default_port = remote_defaults(store)
        host = answer("Remote IP address or DNS hostname")
        port_text = answer("Remote SSH port", default_port)
        try:
            port = int(port_text)
        except ValueError as error:
            raise ManagerError("remote SSH port must be an integer") from error
        user = answer("Remote SSH user", default_user)
        key = Path(answer("Local SSH private-key path", default_key)).expanduser().resolve()
        bootstrap = confirm(
            "If rustup is absent, download the official rustup installer over TLS"
        )
        install_prerequisites = confirm(
            "Install missing remote system build prerequisites with passwordless sudo"
        )
        controller.start_remote(
            target, user, host, port, key, bootstrap, install_prerequisites
        )
        print(f"{target} is running remotely. The SSH session is now detached.")


def check_job(controller: JobController, target: str) -> None:
    status = controller.check(target)
    print(f"{target}: {status}")


def select_job(store: Store, controller: JobController, target: str) -> None:
    row = store.job(target)
    if row["status"] == "pending":
        start_pending(store, controller, target)
    elif row["status"] == "running":
        check_job(controller, target)
    elif row["status"] in {"failed", "unknown"}:
        print(row["message"])
        if confirm(f"Reset {target} to pending and retry"):
            store.reset_job(target)
            start_pending(store, controller, target)
    else:
        print(f"{target} evidence is already verified locally.")


def check_all(store: Store, controller: JobController) -> None:
    for row in store.jobs():
        if row["status"] != "running":
            continue
        try:
            check_job(controller, row["target"])
        except (ManagerError, OSError, subprocess.SubprocessError) as error:
            print(f"{row['target']}: progress check failed: {error}", file=sys.stderr)


def run_menu(store: Store) -> None:
    controller = JobController(store)
    controller.validate_source()
    while True:
        show_menu(store)
        choice = answer("Selection").lower()
        if choice == "q":
            return
        if choice == "c":
            check_all(store, controller)
            continue
        if choice == "f" and store.all_complete():
            controller.finalize()
            print("Final distributed fuzz and native hardware evidence verified.")
            continue
        if choice.isdigit():
            ordinal = int(choice)
            rows = [row for row in store.jobs() if row["ordinal"] == ordinal]
            if rows:
                select_job(store, controller, rows[0]["target"])
                continue
        print("Unknown selection.")


def print_status(store: Store) -> None:
    session = store.session()
    print(f"session={session.identifier}")
    print(f"source_commit={session.source_commit}")
    print(f"collection={session.collection}")
    for row in store.jobs():
        print(f"{row['target']}={row['status']}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state", type=Path, default=DEFAULT_STATE)
    parser.add_argument("--new", action="store_true", help="replace local SQLite state")
    parser.add_argument("--status", action="store_true", help="print stored state without SSH")
    parser.add_argument("--finalize", action="store_true", help="aggregate completed evidence")
    parser.add_argument(
        "--repository",
        default=os.environ.get("BASE64_NG_FUZZ_REPOSITORY", DEFAULT_REPOSITORY),
    )
    arguments = parser.parse_args()
    state = arguments.state.expanduser().resolve()
    try:
        if arguments.new:
            if state.exists():
                existing = Store(state)
                try:
                    if existing.any_running():
                        raise ManagerError(
                            "cannot replace state while jobs are marked running"
                        )
                finally:
                    existing.close()
            remove_database(state)
            store = create_store(state, arguments.repository)
        elif not state.exists():
            if arguments.status or arguments.finalize:
                raise ManagerError(f"session database does not exist: {state}")
            store = create_store(state, arguments.repository)
        elif arguments.status or arguments.finalize:
            store = Store(state)
        else:
            store = open_interactive(state, False, arguments.repository)
        try:
            if arguments.status:
                print_status(store)
            elif arguments.finalize:
                JobController(store).finalize()
            else:
                run_menu(store)
        finally:
            store.close()
    except KeyboardInterrupt:
        print("\nState saved.")
    except (ManagerError, OSError, sqlite3.Error, subprocess.SubprocessError) as error:
        print(f"release evidence manager: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
