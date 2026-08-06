#!/usr/bin/env python3
"""Persistent local and SSH job control for distributed fuzz evidence."""

from __future__ import annotations

import os
import re
import shlex
import sqlite3
import subprocess
import time
import uuid
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_STATE = ROOT / "target" / "fuzz-manager" / "state.sqlite3"
TARGET_FILE = ROOT / "scripts" / "fuzz-release-targets.txt"
FUZZ_SECONDS = 3600
FUZZ_VERSION = (ROOT / "scripts" / "fuzz-cargo-version.txt").read_text().strip()
VALID_USER = re.compile(r"[A-Za-z_][A-Za-z0-9_-]*")
VALID_HOST = re.compile(r"[A-Za-z0-9.-]+")
VALID_REMOTE_PATH = re.compile(r"/[A-Za-z0-9._/-]+")


class ManagerError(RuntimeError):
    pass


@dataclass(frozen=True)
class Source:
    commit: str
    tree: str


@dataclass(frozen=True)
class Session:
    identifier: str
    source_commit: str
    source_tree: str
    collection: Path
    repository: str
    created_at: int


def run_git(*arguments: str, root: Path = ROOT) -> str:
    result = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True, text=True
    )
    return result.stdout.strip()


def source_identity(require_clean: bool = True, root: Path = ROOT) -> Source:
    if require_clean and run_git("status", "--porcelain", "--untracked-files=all", root=root):
        raise ManagerError("fuzz evidence sessions require a clean worktree")
    return Source(
        commit=run_git("rev-parse", "--verify", "HEAD", root=root),
        tree=run_git("rev-parse", "HEAD^{tree}", root=root),
    )


def release_targets() -> list[str]:
    values = [line.strip() for line in TARGET_FILE.read_text().splitlines() if line.strip()]
    if len(values) != 18 or len(values) != len(set(values)):
        raise ManagerError("release fuzz inventory must contain 18 unique targets")
    return values


def validate_remote(user: str, host: str, key_path: Path) -> None:
    if VALID_USER.fullmatch(user) is None:
        raise ManagerError("remote user contains unsupported characters")
    if VALID_HOST.fullmatch(host) is None or ".." in host or host.startswith("-"):
        raise ManagerError("remote host must be an IPv4 address or DNS hostname")
    if not key_path.is_file():
        raise ManagerError(f"SSH private key does not exist: {key_path}")


def validate_remote_work_dir(value: str, expected_prefix: str) -> str:
    path = PurePosixPath(value)
    if (
        VALID_REMOTE_PATH.fullmatch(value) is None
        or not path.is_absolute()
        or ".." in path.parts
        or not path.name.startswith(expected_prefix)
    ):
        raise ManagerError("remote setup returned an invalid work directory")
    return value


class Store:
    def __init__(self, path: Path = DEFAULT_STATE) -> None:
        if path.is_symlink():
            raise ManagerError("refusing a symlinked fuzz-manager database")
        self.path = path.resolve()
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.connection = sqlite3.connect(self.path)
        self.path.chmod(0o600)
        self.connection.row_factory = sqlite3.Row
        self.connection.execute("PRAGMA foreign_keys = ON")
        self.connection.execute("PRAGMA trusted_schema = OFF")
        self.connection.execute("PRAGMA journal_mode = WAL")
        self._schema()

    def close(self) -> None:
        self.connection.close()

    def _schema(self) -> None:
        self.connection.executescript(
            """
            CREATE TABLE IF NOT EXISTS session (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                identifier TEXT NOT NULL,
                source_commit TEXT NOT NULL,
                source_tree TEXT NOT NULL,
                collection TEXT NOT NULL,
                repository TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS jobs (
                target TEXT PRIMARY KEY,
                ordinal INTEGER NOT NULL UNIQUE,
                status TEXT NOT NULL CHECK (
                    status IN ('pending', 'running', 'complete', 'failed', 'unknown')
                ),
                mode TEXT CHECK (mode IN ('local', 'remote') OR mode IS NULL),
                host TEXT,
                remote_user TEXT,
                key_path TEXT,
                work_dir TEXT,
                pid INTEGER,
                started_at INTEGER,
                finished_at INTEGER,
                exit_code INTEGER,
                message TEXT NOT NULL DEFAULT ''
            );
            """
        )
        self.connection.commit()

    def has_session(self) -> bool:
        return self.connection.execute("SELECT 1 FROM session").fetchone() is not None

    def create_session(self, source: Source, repository: str) -> Session:
        if self.has_session():
            raise ManagerError("the state database already contains a session")
        identifier = f"{source.commit[:12]}-{uuid.uuid4().hex[:8]}"
        collection = (self.path.parent / "sessions" / identifier / "shards").resolve()
        collection.mkdir(parents=True)
        now = int(time.time())
        with self.connection:
            self.connection.execute(
                "INSERT INTO session VALUES (1, ?, ?, ?, ?, ?, ?)",
                (identifier, source.commit, source.tree, str(collection), repository, now),
            )
            self.connection.executemany(
                "INSERT INTO jobs(target, ordinal, status) VALUES (?, ?, 'pending')",
                ((target, index) for index, target in enumerate(release_targets(), start=1)),
            )
        return self.session()

    def session(self) -> Session:
        row = self.connection.execute("SELECT * FROM session WHERE singleton = 1").fetchone()
        if row is None:
            raise ManagerError("no fuzz evidence session exists")
        return Session(
            identifier=row["identifier"],
            source_commit=row["source_commit"],
            source_tree=row["source_tree"],
            collection=Path(row["collection"]),
            repository=row["repository"],
            created_at=row["created_at"],
        )

    def jobs(self) -> list[sqlite3.Row]:
        return list(self.connection.execute("SELECT * FROM jobs ORDER BY ordinal"))

    def job(self, target: str) -> sqlite3.Row:
        row = self.connection.execute("SELECT * FROM jobs WHERE target = ?", (target,)).fetchone()
        if row is None:
            raise ManagerError(f"unknown fuzz target: {target}")
        return row

    def update(self, target: str, **values: object) -> None:
        allowed = {
            "status",
            "mode",
            "host",
            "remote_user",
            "key_path",
            "work_dir",
            "pid",
            "started_at",
            "finished_at",
            "exit_code",
            "message",
        }
        if not values or not set(values).issubset(allowed):
            raise ManagerError("invalid or empty job update")
        assignments = ", ".join(f"{key} = ?" for key in values)
        with self.connection:
            self.connection.execute(
                f"UPDATE jobs SET {assignments} WHERE target = ?",  # noqa: S608
                (*values.values(), target),
            )

    def reset_job(self, target: str) -> None:
        with self.connection:
            self.connection.execute(
                """UPDATE jobs SET status='pending', mode=NULL, host=NULL,
                remote_user=NULL, key_path=NULL, work_dir=NULL, pid=NULL,
                started_at=NULL, finished_at=NULL, exit_code=NULL, message=''
                WHERE target=?""",
                (target,),
            )

    def local_running(self) -> bool:
        return (
            self.connection.execute(
                "SELECT 1 FROM jobs WHERE status='running' AND mode='local'"
            ).fetchone()
            is not None
        )

    def remote_host_running(self, host: str) -> bool:
        return (
            self.connection.execute(
                "SELECT 1 FROM jobs WHERE status='running' AND mode='remote' AND host=?",
                (host,),
            ).fetchone()
            is not None
        )

    def any_running(self) -> bool:
        return (
            self.connection.execute(
                "SELECT 1 FROM jobs WHERE status='running'"
            ).fetchone()
            is not None
        )

    def all_complete(self) -> bool:
        row = self.connection.execute(
            "SELECT COUNT(*) AS count FROM jobs WHERE status != 'complete'"
        ).fetchone()
        return row["count"] == 0

    def last_remote(self) -> sqlite3.Row | None:
        return self.connection.execute(
            """SELECT remote_user, key_path FROM jobs
            WHERE mode='remote' ORDER BY started_at DESC LIMIT 1"""
        ).fetchone()


def ssh_command(user: str, host: str, key_path: Path) -> list[str]:
    validate_remote(user, host, key_path)
    return [
        "ssh",
        "-i",
        str(key_path),
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "ConnectTimeout=15",
        "-o",
        "ServerAliveInterval=30",
        f"{user}@{host}",
    ]


def scp_command(user: str, host: str, key_path: Path) -> list[str]:
    validate_remote(user, host, key_path)
    return [
        "scp",
        "-i",
        str(key_path),
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "ConnectTimeout=15",
    ]


def write_local_runner(path: Path, target: str, collection: Path, label: str) -> None:
    command = (
        f"BASE64_NG_FUZZ_MACHINE_LABEL={shlex.quote(label)} "
        f"scripts/capture-fuzz-shard.sh {shlex.quote(target)} "
        f"{shlex.quote(str(collection))} {FUZZ_SECONDS}"
    )
    path.write_text(
        "#!/usr/bin/env sh\nset +e\n"
        'printf "%s\\n" "$$" > "$2/pid"\n'
        'cleanup_lock() { rm -f "$2/pid"; rmdir "$2" 2>/dev/null || true; }\n'
        "trap cleanup_lock EXIT INT TERM\n"
        f"cd {shlex.quote(str(ROOT))}\n{command}\n"
        "status=$?\n"
        'printf "%s\\n" "$status" > "$1.tmp"\n'
        'mv "$1.tmp" "$1"\nexit "$status"\n'
    )
    path.chmod(0o700)


def pid_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def acquire_local_lock() -> Path:
    lock = ROOT / "target" / "fuzz-manager" / "local-active"
    lock.parent.mkdir(parents=True, exist_ok=True)
    try:
        lock.mkdir()
        return lock
    except FileExistsError:
        owner = lock / "pid"
        try:
            pid = int(owner.read_text().strip())
        except (OSError, ValueError):
            raise ManagerError("a local fuzz launch is already in progress") from None
        if pid_alive(pid):
            raise ManagerError(f"local fuzz process {pid} is already running")
        owner.unlink(missing_ok=True)
        lock.rmdir()
        lock.mkdir()
        return lock
