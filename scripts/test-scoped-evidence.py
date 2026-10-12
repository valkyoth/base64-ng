#!/usr/bin/env python3
"""Hostile transport and complete-inventory regressions; no real packets needed."""

import copy
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import socket
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import scoped_evidence as evidence


def digest(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def compress(raw):
    return gzip.compress(raw, mtime=0)


class PacketTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="base64-scoped-packet-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.path = self.root / "packet.tar.gz"
        self.entries = [("capture", None, tarfile.DIRTYPE),
                        ("capture/result.json", b'{"status":"passed"}', tarfile.REGTYPE)]
        self.manifest = {"schema": 1, "source": "a" * 40, "release_authorized": False,
                         "files": {"capture/result.json": digest(self.entries[1][1])},
                         "directories": ["capture"]}

    def packet(self, raw_manifest=None, metadata=None):
        raw = raw_manifest if raw_manifest is not None else json.dumps(self.manifest).encode()
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
            for name, data, kind in self.entries + [("MANIFEST.json", raw, tarfile.REGTYPE)]:
                member = tarfile.TarInfo(name)
                member.type = kind
                member.mode = 0o700 if kind == tarfile.DIRTYPE else 0o600
                if kind in (tarfile.LNKTYPE, tarfile.SYMTYPE):
                    member.linkname = "capture/result.json"
                member.size = len(data) if data is not None else 0
                for key, value in (metadata or {}).items():
                    setattr(member, key, value)
                archive.addfile(member, io.BytesIO(data) if data is not None else None)
        self.path.write_bytes(compress(buffer.getvalue()))
        return {"archive": digest(self.path.read_bytes()), "manifest": digest(raw),
                "gzip_header": self.path.read_bytes()[:10].hex(),
                "implicit_directories": [],
                "file_count": len(self.manifest["files"]),
                "directory_count": len(self.manifest["directories"]),
                "metadata": {k: v for k, v in self.manifest.items() if k not in ("files", "directories")}}

    def rejects(self, reason, pin=None):
        with self.assertRaisesRegex((ValueError, OSError), reason):
            evidence.verify_packet(self.path, self.packet() if pin is None else pin)

    def test_complete_packet(self):
        self.assertEqual(evidence.verify_packet(self.path, self.packet()), self.manifest)

    def test_uninventoried_tar_metadata_even_with_repinned_transport(self):
        for field, value in (("uid", 1), ("gid", 2), ("uname", "hidden"),
                             ("gname", "hidden"), ("mtime", 1), ("mode", 0o777),
                             ("linkname", "hidden"),
                             ("pax_headers", {"comment": "HIDDEN-UNINVENTORIED-DATA"}),
                             ("pax_headers", {"SCHILY.xattr.secret": "hidden"}),
                             ("pax_headers", {"path": "capture/"})):
            with self.subTest(field=field, value=value):
                self.rejects("metadata", self.packet(metadata={field: value}))

    def test_canonical_long_and_unicode_paths(self):
        for name in ("x" * 180, "\u00e9" * 60):
            for kind, data in ((tarfile.DIRTYPE, None), (tarfile.REGTYPE, b"valid")):
                with self.subTest(name=name, kind=kind):
                    self.entries = [(name, data, kind)]
                    self.manifest["files"] = {name: digest(data)} if data is not None else {}
                    self.manifest["directories"] = [name] if data is None else []
                    self.assertEqual(evidence.verify_packet(self.path, self.packet()), self.manifest)

    def test_pax_raw_metadata_padding_duplicates_and_header_chains(self):
        name = "x" * 180
        self.entries = [(name, b"valid", tarfile.REGTYPE)]
        self.manifest["files"] = {name: digest(b"valid")}
        self.manifest["directories"] = []
        for mutation in ("padding", "owner", "duplicate", "global", "chain", "missing", "oversized"):
            with self.subTest(mutation=mutation):
                pin = self.packet()
                raw = gzip.decompress(self.path.read_bytes())
                header = tarfile.TarInfo.frombuf(raw[:512], "utf-8", "strict")
                payload = raw[512:512 + header.size]
                if mutation == "padding":
                    raw = raw[:512 + header.size] + b"X" + raw[513 + header.size:]
                elif mutation == "owner":
                    header.uname = "hidden"
                    raw = header.tobuf(tarfile.USTAR_FORMAT) + raw[512:]
                elif mutation == "duplicate":
                    header.size *= 2
                    duplicated = payload * 2
                    raw = (header.tobuf(tarfile.USTAR_FORMAT) + duplicated
                           + bytes(-len(duplicated) % 512) + raw[1024:])
                elif mutation == "global":
                    header.type = tarfile.XGLTYPE
                    raw = header.tobuf(tarfile.USTAR_FORMAT) + raw[512:]
                elif mutation == "chain":
                    raw = raw[:1024] + raw
                elif mutation == "missing":
                    raw = raw[:1024] + bytes(1024)
                else:
                    header.size = evidence.MAX_PAX_PATH + 1
                    raw = header.tobuf(tarfile.USTAR_FORMAT) + raw[512:]
                self.repin_transport(pin, compress(raw))
                self.rejects("metadata|special archive|missing member", pin)

    def test_unused_tar_header_bytes_are_not_a_hidden_channel(self):
        for offset, value in ((500, b"hidden"), (329, b"0000003\0"),
                              (337, b"0000004\0"), (20, b"hidden")):
            with self.subTest(offset=offset):
                pin = self.packet()
                raw = bytearray(gzip.decompress(self.path.read_bytes()))
                raw[offset:offset + len(value)] = value
                raw[148:156] = b"        "
                raw[148:156] = (f"{sum(raw[:512]):06o}\0 ").encode()
                self.repin_transport(pin, compress(raw))
                self.rejects("metadata", pin)

    def test_gzip_optional_metadata_and_timestamp_require_separate_pin(self):
        for flag, extra in ((8, b"hidden\0"), (16, b"comment\0"),
                            (4, b"\x06\x00hidden"), (2, b"\0\0"), (32, b"")):
            with self.subTest(flag=flag):
                pin = self.packet()
                raw = bytearray(self.path.read_bytes())
                raw[3] = flag
                self.repin_transport(pin, raw[:10] + extra + raw[10:])
                self.rejects("gzip metadata", pin)
        pin = self.packet()
        raw = bytearray(self.path.read_bytes())
        raw[4] = 1
        self.repin_transport(pin, raw)
        self.rejects("gzip metadata", pin)

    def test_reviewed_gzip_filename_and_timestamp_are_preserved(self):
        pin = self.packet()
        buffer = io.BytesIO()
        with gzip.GzipFile(fileobj=buffer, filename="review-packet.tar.gz.partial", mode="wb",
                           mtime=1791811205) as archive:
            archive.write(gzip.decompress(self.path.read_bytes()))
        raw = buffer.getvalue()
        pin["gzip_header"] = raw[:10 + len("review-packet.tar.gz.partial") + 1].hex()
        self.repin_transport(pin, raw)
        self.assertEqual(evidence.verify_packet(self.path, pin), self.manifest)
        self.repin_transport(pin, raw.replace(b"review-packet", b"hidden-packet", 1))
        self.rejects("gzip metadata", pin)

    def test_gzip_metadata_pin_is_required_and_restricted(self):
        for value in (None, "", "zz", "0" * 513, "1f8b08000000000002ff00",
                      "1f8b08100000000002ff", "1f8b08080000000002ff2f746d7000"):
            with self.subTest(value=value):
                pin = self.packet()
                pin["gzip_header"] = value
                self.rejects("gzip", pin)

    def repin_transport(self, pin, raw):
        self.path.write_bytes(raw)
        pin["archive"] = digest(raw)
        return pin

    def test_raw_trailing_data_even_when_transport_is_repinned(self):
        for trailing in (b"HIDDEN-UNINVENTORIED-DATA", bytes(64)):
            with self.subTest(trailing=trailing):
                pin = self.packet()
                self.repin_transport(pin, self.path.read_bytes() + trailing)
                self.rejects("gzip EOF", pin)

    def test_concatenated_gzip_members_including_empty_member(self):
        for payload in (b"hidden", b"", bytes(1024)):
            with self.subTest(payload=payload):
                pin = self.packet()
                self.repin_transport(pin, self.path.read_bytes() + compress(payload))
                self.rejects("gzip EOF", pin)

    def test_data_after_tar_terminator_including_prefetch_buffer(self):
        for tail in (b"hidden", b"X" * 512, b"X" * 20000):
            with self.subTest(length=len(tail)):
                pin = self.packet()
                decoded = gzip.decompress(self.path.read_bytes())
                # Five blocks precede the end marker in this fixture. Place
                # hidden bytes inside the first 10-KiB TarFile read buffer.
                for offset in (6 * 512, len(decoded)):
                    self.repin_transport(pin, compress(decoded[:offset] + tail))
                    self.rejects("after tar end", pin)

    def test_second_tar_and_excessive_zero_padding(self):
        for tail_kind in ("second-tar", "zero-bomb"):
            with self.subTest(tail=tail_kind):
                pin = self.packet()
                decoded = gzip.decompress(self.path.read_bytes())
                tail = decoded if tail_kind == "second-tar" else bytes(2 * 1024**2)
                self.repin_transport(pin, compress(decoded + tail))
                self.rejects("after tar end", pin)

    def test_gzip_crc_and_truncated_trailer(self):
        for mutation in ("crc", "truncate"):
            with self.subTest(mutation=mutation):
                pin = self.packet()
                raw = bytearray(self.path.read_bytes())
                if mutation == "crc":
                    raw[-8] ^= 1
                else:
                    del raw[-4:]
                self.repin_transport(pin, raw)
                self.rejects("gzip", pin)

    def test_missing_partial_or_invalid_tar_terminator(self):
        for tail in (b"", bytes(511), bytes(512), b"X" * 512):
            with self.subTest(length=len(tail)):
                pin = self.packet()
                decoded = gzip.decompress(self.path.read_bytes())
                self.repin_transport(pin, compress(decoded[:5 * 512] + tail))
                with self.assertRaises((ValueError, tarfile.ReadError)):
                    evidence.verify_packet(self.path, pin)

    def test_nonzero_member_padding(self):
        pin = self.packet()
        decoded = bytearray(gzip.decompress(self.path.read_bytes()))
        decoded[2 * 512 + len(self.entries[1][1])] = 1
        self.repin_transport(pin, compress(decoded))
        self.rejects("member padding", pin)

    def test_minimum_and_maximum_legal_tar_padding(self):
        for size in (2 * 512, evidence.MAX_TAR_PADDING):
            with self.subTest(size=size):
                pin = self.packet()
                payload = gzip.decompress(self.path.read_bytes())[:5 * 512]
                self.repin_transport(pin, compress(payload + bytes(size)))
                self.assertEqual(evidence.verify_packet(self.path, pin), self.manifest)

    def test_transport_is_hashed_before_tar_parsing(self):
        pin = self.packet()
        self.path.write_bytes(b"not an archive")
        with patch.object(tarfile, "open", side_effect=AssertionError("must not parse")):
            self.rejects("transport", pin)

    def test_manifest_pin(self):
        pin = self.packet()
        pin["manifest"]["sha256"] = "0" * 64
        self.rejects("manifest digest", pin)

    def test_changed_or_missing_indexed_bytes(self):
        for mutation in ("changed", "missing"):
            with self.subTest(mutation=mutation):
                self.entries = [("capture", None, tarfile.DIRTYPE)]
                if mutation == "changed":
                    self.entries.append(("capture/result.json", b"changed", tarfile.REGTYPE))
                self.rejects("inventory")

    def test_extra_file_and_empty_directory(self):
        for kind, data in ((tarfile.REGTYPE, b""), (tarfile.DIRTYPE, None)):
            with self.subTest(kind=kind):
                self.entries.append(("extra", data, kind))
                self.rejects("inventory")
                self.entries.pop()

    def test_duplicate_entries(self):
        self.entries.append(self.entries[1])
        self.rejects("duplicate member")

    def test_duplicate_manifest(self):
        self.entries.append(("MANIFEST.json", b"{}", tarfile.REGTYPE))
        self.rejects("duplicate member")

    def test_links_and_special_members(self):
        for kind in (tarfile.LNKTYPE, tarfile.SYMTYPE, tarfile.FIFOTYPE,
                     tarfile.CHRTYPE, tarfile.BLKTYPE, b"s"):
            with self.subTest(kind=kind):
                self.entries.append(("hostile", None, kind))
                self.rejects("special archive")
                self.entries.pop()

    def test_noncanonical_paths(self):
        for name in ("../escape", "/absolute", "a/../b", "a//b", "./a", "a\\b", "a\nname", "."):
            with self.subTest(name=name):
                self.entries.append((name, b"", tarfile.REGTYPE))
                self.rejects("canonical")
                self.entries.pop()

    def test_manifest_json_duplicate_keys_and_nonfinite_values(self):
        for raw in (b'{"files":{},"files":{}}', b'{"value":NaN}', b'{"value":Infinity}'):
            with self.subTest(raw=raw):
                self.rejects("duplicate JSON|non-finite", self.packet(raw))

    def test_record_types(self):
        for value in (True, -1, 1.0, "1"):
            with self.subTest(value=value):
                self.manifest["files"]["capture/result.json"]["bytes"] = value
                self.rejects("size/hash")

    def test_directory_inventory_duplicates(self):
        self.manifest["directories"].append("capture")
        self.rejects("inventory")

    def test_parent_must_be_present_directory(self):
        self.entries.pop(0)
        self.manifest["directories"] = []
        self.rejects("archive ancestor")

    def test_only_exact_pinned_implicit_parents_are_accepted(self):
        self.entries.pop(0)
        self.manifest["directories"] = []
        pin = self.packet()
        pin["implicit_directories"] = ["capture"]
        self.assertEqual(evidence.verify_packet(self.path, pin), self.manifest)
        pin["implicit_directories"] = ["capture", "extra"]
        self.rejects("implicit archive ancestor", pin)

    def test_parent_cannot_be_file(self):
        self.entries[0] = ("capture", b"", tarfile.REGTYPE)
        self.manifest["directories"] = []
        self.manifest["files"]["capture"] = digest(b"")
        self.rejects("archive ancestor")

    def test_source_status_and_count_pins(self):
        original = self.packet()
        for field in ("source", "release_authorized", "file_count", "directory_count"):
            with self.subTest(field=field):
                pin = copy.deepcopy(original)
                if field in pin:
                    pin[field] += 1
                else:
                    pin["metadata"][field] = "changed"
                self.rejects("metadata|inventory", pin)

    def test_entry_and_expanded_byte_limits(self):
        pin = self.packet()
        with patch.object(evidence, "MAX_ENTRIES", 1):
            self.rejects("entry limit", pin)
        self.entries.append(("large", b"x" * 8192, tarfile.REGTYPE))
        pin = self.packet()
        with patch.object(evidence, "MAX_BYTES", 2048):
            self.rejects("expanded archive", pin)

    def test_manifest_limit(self):
        pin = self.packet()
        with patch.object(evidence, "MAX_MANIFEST", 1):
            self.rejects("manifest exceeds", pin)

    def test_archive_input_links(self):
        pin = self.packet()
        alias = self.root / "alias"
        alias.hardlink_to(self.path)
        self.rejects("linked", pin)
        alias.unlink()
        self.path.rename(alias)
        self.path.symlink_to(alias)
        self.rejects("symbolic|links", pin)

    def test_fifo_and_socket_inputs_do_not_block(self):
        pin = self.packet()
        self.path.unlink()
        os.mkfifo(self.path)
        self.rejects("non-regular", pin)
        self.path.unlink()
        with socket.socket(socket.AF_UNIX) as sock:
            sock.bind(str(self.path))
            self.rejects("address|device", pin)

    def test_symlink_ancestor(self):
        pin = self.packet()
        alias = self.root / "alias"
        alias.symlink_to(self.root, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "ancestor"):
            evidence.verify_packet(alias / self.path.name, pin)

    def test_changed_input_during_read(self):
        self.packet()
        with self.assertRaisesRegex(ValueError, "changed during"):
            with evidence.regular_file(self.path):
                self.path.write_bytes(b"replacement")


if __name__ == "__main__":
    unittest.main()
