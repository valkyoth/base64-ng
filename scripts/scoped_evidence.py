"""Read-only verification of pinned private review transports, never extraction."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import tarfile
import zlib


MAX_ENTRIES = 10_000
MAX_BYTES = 2 * 1024**3
MAX_MANIFEST = 2 * 1024**2
MAX_TAR_PADDING = tarfile.RECORDSIZE + tarfile.BLOCKSIZE
MAX_PAX_PATH = 4096
MAX_PADDED_PAX = ((MAX_PAX_PATH + tarfile.BLOCKSIZE - 1) // tarfile.BLOCKSIZE) * tarfile.BLOCKSIZE
# PAX header, padded path record, member header, and worst-case payload padding.
MAX_ENTRY_OVERHEAD = 2 * tarfile.BLOCKSIZE + MAX_PADDED_PAX + (tarfile.BLOCKSIZE - 1)
SHA256 = re.compile(r"[0-9a-f]{64}")


class SingleGzipReader:
    """Bounded streaming decompression with exactly one fully consumed gzip member."""

    def __init__(self, stream):
        self.stream = stream
        self.decoder = zlib.decompressobj(16 + zlib.MAX_WBITS)
        self.pending = b""
        self.total = 0

    def read(self, size):
        if not 0 <= size <= 16 * 1024**2:
            raise ValueError("invalid decompression read size")
        output = bytearray()
        while len(output) < size:
            if self.decoder.eof:
                if self.decoder.unused_data or self.stream.read(1):
                    raise ValueError("data or extra gzip member after gzip EOF")
                break
            data = self.pending or self.stream.read(64 * 1024)
            if not data:
                raise ValueError("truncated gzip stream")
            try:
                chunk = self.decoder.decompress(data, size - len(output))
            except zlib.error as error:
                raise ValueError("invalid gzip stream or checksum") from error
            self.pending = self.decoder.unconsumed_tail
            output.extend(chunk)
            self.total += len(chunk)
            if self.total > MAX_BYTES + MAX_ENTRIES * MAX_ENTRY_OVERHEAD + MAX_TAR_PADDING:
                raise ValueError("expanded transport exceeds byte limit")
        return bytes(output)


class StrictTarInfo(tarfile.TarInfo):
    @classmethod
    def fromtarfile(cls, archive):
        # Accept only the canonical writer's regular/directory headers and one
        # necessary PAX path record. Generic PAX parsing silently loses duplicate
        # keys, extension-header metadata and padding before yielding a member.
        offset = archive.fileobj.tell()
        raw = archive.fileobj.read(tarfile.BLOCKSIZE)
        member = cls.frombuf(raw, "utf-8", "strict")
        path = None
        if member.type == tarfile.XHDTYPE:
            if not 0 < member.size <= MAX_PAX_PATH:
                raise ValueError("PAX path exceeds metadata limit")
            payload = archive.fileobj.read(member.size + (-member.size % tarfile.BLOCKSIZE))
            record = re.fullmatch(rb"([1-9][0-9]*) path=(.+)\n", payload[:member.size])
            if record is None or int(record[1]) != member.size:
                raise ValueError("non-canonical PAX path metadata")
            path = record[2].decode("utf-8", "strict")
            header = archive.fileobj.read(tarfile.BLOCKSIZE)
            raw += payload + header
            # A terminator here is not the end of the archive: a PAX record
            # must be followed by its own regular/directory header.
            if header == bytes(tarfile.BLOCKSIZE):
                raise ValueError("missing member after PAX path")
            member = cls.frombuf(header, "utf-8", "strict")
        if member.type not in (tarfile.REGTYPE, tarfile.DIRTYPE) or member.size < 0:
            raise ValueError("linked, sparse or special archive member")
        if path is not None:
            member.name = path.rstrip("/") if member.isdir() else path
        canonical(member.name)
        expected = tarfile.TarInfo(member.name)
        expected.type = member.type
        expected.mode = 0o700 if member.isdir() else 0o600
        expected.size = member.size
        if raw != expected.tobuf(format=tarfile.PAX_FORMAT, encoding="utf-8", errors="strict"):
            raise ValueError("non-canonical tar metadata")
        member.offset = offset
        member.offset_data = archive.fileobj.tell()
        archive.offset = member.offset_data + member.size + (-member.size % tarfile.BLOCKSIZE)
        return member

    @classmethod
    def frombuf(cls, buf, encoding, errors):
        # TarFile normally tolerates an invalid/truncated header after a member.
        # Only a complete zero block may terminate this evidence archive.
        if buf == bytes(tarfile.BLOCKSIZE):
            raise tarfile.EOFHeaderError("end marker")
        try:
            return super().frombuf(buf, encoding, errors)
        except tarfile.HeaderError as error:
            raise tarfile.ReadError("invalid or missing tar end marker") from error


def finish_tar(archive):
    # Drain through TarFile's stream, not its underlying reader: the former may
    # already hold prefetched bytes after the first tar end marker.
    if archive.fileobj.tell() != archive.offset + tarfile.BLOCKSIZE:
        raise ValueError("missing tar end marker")
    padding = archive.fileobj.read(MAX_TAR_PADDING)
    if (len(padding) < tarfile.BLOCKSIZE
            or len(padding) > MAX_TAR_PADDING - tarfile.BLOCKSIZE
            or len(padding) % tarfile.BLOCKSIZE or any(padding)):
        raise ValueError("invalid or excessive data after tar end marker")
    if archive.fileobj.read(1):
        raise ValueError("data after tar end marker")


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


def json_record(value):
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def selected_inventory(manifest, path):
    canonical(path)
    files = {name: record for name, record in manifest["files"].items()
             if name == path or name.startswith(path + "/")}
    if not files:
        raise ValueError("missing or empty attested evidence inventory")
    directories = [name for name in manifest["directories"]
                   if name == path or name.startswith(path + "/")]
    return {"files": files, "directories": directories}


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


def verify_gzip_header(stream, pin):
    # Existing transports contain their writer's basename and timestamp. Keep
    # those bytes unchanged, but bind them explicitly rather than accepting any
    # optional gzip metadata under a newly computed outer archive hash.
    value = pin.get("gzip_header")
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{20,512}", value) or len(value) % 2:
        raise ValueError("invalid pinned gzip metadata")
    header = bytes.fromhex(value)
    if (header[:3] != b"\x1f\x8b\x08" or header[3] not in (0, 8)
            or header[8] != 2 or header[9] not in (3, 255)):
        raise ValueError("unsupported gzip metadata")
    if header[3] == 8:
        if not re.fullmatch(rb"[A-Za-z0-9][A-Za-z0-9._-]*\x00", header[10:]):
            raise ValueError("invalid pinned gzip filename")
    elif len(header) != 10:
        raise ValueError("unexpected pinned gzip metadata")
    if stream.read(len(header)) != header:
        raise ValueError("gzip metadata differs from reviewed pin")
    stream.seek(0)


def verify_packet(path, pin):
    """Hash before parsing; then verify every member against the pinned manifest."""
    require_record(pin["archive"])
    require_record(pin["manifest"])
    with regular_file(path) as stream:
        if digest_stream(stream, MAX_BYTES) != pin["archive"]:
            raise ValueError("packet transport digest/size mismatch")
        stream.seek(0)
        verify_gzip_header(stream, pin)
        files, directories, seen = {}, [], set()
        total, manifest_raw = 0, None
        decoded = SingleGzipReader(stream)
        with tarfile.open(fileobj=decoded, mode="r|", tarinfo=StrictTarInfo) as archive:
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
                padding = archive.fileobj.read(-member.size % tarfile.BLOCKSIZE)
                if any(padding):
                    raise ValueError("non-zero member padding")
            finish_tar(archive)
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
