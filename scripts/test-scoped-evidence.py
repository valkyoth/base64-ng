#!/usr/bin/env python3
"""Hostile transport and complete-inventory regressions; no real packets needed."""

import copy
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

    def packet(self, raw_manifest=None):
        raw = raw_manifest if raw_manifest is not None else json.dumps(self.manifest).encode()
        with tarfile.open(self.path, "w:gz") as archive:
            for name, data, kind in self.entries + [("MANIFEST.json", raw, tarfile.REGTYPE)]:
                member = tarfile.TarInfo(name)
                member.type = kind
                if kind in (tarfile.LNKTYPE, tarfile.SYMTYPE):
                    member.linkname = "capture/result.json"
                member.size = len(data) if data is not None else 0
                archive.addfile(member, io.BytesIO(data) if data is not None else None)
        return {"archive": digest(self.path.read_bytes()), "manifest": digest(raw),
                "implicit_directories": [],
                "file_count": len(self.manifest["files"]),
                "directory_count": len(self.manifest["directories"]),
                "metadata": {k: v for k, v in self.manifest.items() if k not in ("files", "directories")}}

    def rejects(self, reason, pin=None):
        with self.assertRaisesRegex((ValueError, OSError), reason):
            evidence.verify_packet(self.path, self.packet() if pin is None else pin)

    def test_complete_packet(self):
        self.assertEqual(evidence.verify_packet(self.path, self.packet()), self.manifest)

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
