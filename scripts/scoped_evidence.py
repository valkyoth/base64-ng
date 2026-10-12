"""Read-only verification of pinned private review transports, never extraction."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import tarfile


MAX_ENTRIES = 10_000
MAX_BYTES = 2 * 1024**3
MAX_MANIFEST = 2 * 1024**2
SHA256 = re.compile(r"[0-9a-f]{64}")


def canonical(name):
    if not isinstance(name, str) or not name:
        raise ValueError("empty or non-string artifact path")
    path = PurePosixPath(name)
    if (not path.parts or path.is_absolute() or path.as_posix() != name
            or ".." in path.parts or "\\" in name
            or any(ord(char) < 32 or ord(char) == 127 for char in name)):
        raise ValueError("non-canonical artifact path: " + repr(name))
    return path


def strict_json(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON key: " + key)
            result[key] = value
        return result

    def invalid(value):
        raise ValueError("non-finite JSON value: " + value)

    return json.loads(raw, object_pairs_hook=unique, parse_constant=invalid)


@contextmanager
def regular_file(path, limit=MAX_BYTES):
    path = Path(path).absolute()
    for parent in path.parents:
        if not stat.S_ISDIR(parent.lstat().st_mode):
            raise ValueError("non-directory or linked ancestor: " + str(parent))
    # O_NONBLOCK prevents a pre-positioned FIFO from blocking before fstat.
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), "rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
            raise ValueError("linked or non-regular input: " + str(path))
        if before.st_size > limit:
            raise ValueError("input exceeds byte limit")
        yield stream
        fields = ("st_dev", "st_ino", "st_mode", "st_nlink", "st_size", "st_mtime_ns", "st_ctime_ns")
        for after in (os.fstat(stream.fileno()), path.lstat()):
            if any(getattr(before, key) != getattr(after, key) for key in fields):
                raise ValueError("input changed during verification")


def read_regular(path, limit=MAX_MANIFEST):
    with regular_file(path, limit) as stream:
        raw = stream.read(limit + 1)
        if len(raw) > limit:
            raise ValueError("input grew beyond byte limit")
        return raw


def digest_stream(stream, limit):
    digest, size = hashlib.sha256(), 0
    while chunk := stream.read(min(1024**2, limit - size + 1)):
        size += len(chunk)
        if size > limit:
            raise ValueError("stream exceeds byte limit")
        digest.update(chunk)
    return {"bytes": size, "sha256": digest.hexdigest()}


def require_record(record):
    if (not isinstance(record, dict) or set(record) != {"bytes", "sha256"}
            or type(record["bytes"]) is not int or not 0 <= record["bytes"] <= MAX_BYTES
            or not isinstance(record["sha256"], str) or not SHA256.fullmatch(record["sha256"])):
        raise ValueError("invalid size/hash record")


def verify_packet(path, pin):
    """Hash before parsing; then verify every member against the pinned manifest."""
    require_record(pin["archive"])
    require_record(pin["manifest"])
    with regular_file(path) as stream:
        if digest_stream(stream, MAX_BYTES) != pin["archive"]:
            raise ValueError("packet transport digest/size mismatch")
        stream.seek(0)
        files, directories, seen = {}, [], set()
        total, manifest_raw = 0, None
        with tarfile.open(fileobj=stream, mode="r|gz") as archive:
            for member in archive:
                canonical(member.name)
                if member.name in seen or len(seen) >= MAX_ENTRIES:
                    raise ValueError("duplicate member or entry limit exceeded")
                seen.add(member.name)
                if member.size < 0 or member.issparse() or not (member.isfile() or member.isdir()):
                    raise ValueError("linked, sparse or special archive member")
                total += member.size
                if total > MAX_BYTES:
                    raise ValueError("expanded archive exceeds byte limit")
                if member.isdir():
                    if member.size:
                        raise ValueError("directory has a payload")
                    directories.append(member.name)
                    continue
                with archive.extractfile(member) as incoming:
                    if member.name == "MANIFEST.json":
                        if member.size > MAX_MANIFEST:
                            raise ValueError("manifest exceeds byte limit")
                        manifest_raw = incoming.read(MAX_MANIFEST + 1)
                        if len(manifest_raw) != member.size:
                            raise ValueError("truncated manifest")
                    else:
                        files[member.name] = digest_stream(incoming, member.size)
                        if files[member.name]["bytes"] != member.size:
                            raise ValueError("truncated archive member")
        if manifest_raw is None:
            raise ValueError("missing manifest")
        actual = {"bytes": len(manifest_raw), "sha256": hashlib.sha256(manifest_raw).hexdigest()}
        if actual != pin["manifest"]:
            raise ValueError("packet manifest digest/size mismatch")
        manifest = strict_json(manifest_raw)
        expected_files = manifest["files"]
        if not isinstance(expected_files, dict):
            raise ValueError("invalid manifest files")
        for name, record in expected_files.items():
            canonical(name)
            require_record(record)
        expected_dirs = manifest["directories"]
        if not isinstance(expected_dirs, list) or any(not isinstance(x, str) for x in expected_dirs):
            raise ValueError("invalid directory inventory")
        if (files != expected_files or sorted(directories) != expected_dirs
                or len(files) != pin["file_count"] or len(directories) != pin["directory_count"]):
            raise ValueError("complete packet inventory mismatch")
        directory_set = set(directories)
        implicit = set()
        for name in seen:
            for parent in canonical(name).parents:
                if str(parent) != "." and str(parent) not in directory_set:
                    if str(parent) in seen:
                        raise ValueError("non-directory archive ancestor")
                    implicit.add(str(parent))
        # The original reviewed tar omitted one explicit parent directory entry.
        # Preserve that transport byte-for-byte; never generalize this to files
        # or filesystem inventories, and never materialize these paths here.
        if sorted(implicit) != pin["implicit_directories"]:
            raise ValueError("missing or unexpected implicit archive ancestor")
        metadata = {key: value for key, value in manifest.items() if key not in ("files", "directories")}
        if metadata != pin["metadata"]:
            raise ValueError("packet source/status metadata mismatch")
    return manifest
