#!/usr/bin/env python3
"""Regression tests for exact-source native release evidence."""

from __future__ import annotations

import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = runpy.run_path(str(ROOT / "scripts/test-neon-admission-bundle.py"))


def write_campaign(root: Path) -> str:
    root.mkdir(parents=True)
    for platform in ("apple-silicon", "aarch64-linux"):
        bundle = root / platform
        FIXTURE["write_bundle"](bundle)
        if platform == "apple-silicon":
            manifest = bundle / "MANIFEST.txt"
            manifest.write_text(manifest.read_text().replace(
                "host=aarch64-unknown-linux-gnu", "host=aarch64-apple-darwin"))
            (bundle / "cpu.txt").write_text(
                "hw.ncpu: 8\nhw.byteorder: 1234\nhw.optional.neon: 1\n"
                "hw.optional.arm64: 1\nmachdep.cpu.brand_string: fixture\n")
            (bundle / "uname.txt").write_text("Darwin 27.0.0 arm64\n")
            FIXTURE["write_checksums"](bundle)
    return subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()


class NativeCampaignTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "neon"
        self.source = write_campaign(self.root)
        self.apple = self.root / "apple-silicon"

    def run_gate(self, success, *, root=None, source=None):
        result = subprocess.run([
            sys.executable, "scripts/validate-neon-campaign.py", str(root or self.root),
            "--source", source or self.source,
        ], cwd=ROOT, text=True, capture_output=True, timeout=30)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)

    def rewrite(self, old, new):
        path = self.apple / "MANIFEST.txt"
        path.write_text(path.read_text().replace(old, new))
        FIXTURE["write_checksums"](self.apple)

    def test_accepts_both_native_platforms(self):
        self.run_gate(True)

    def test_rejects_missing_platform(self):
        shutil.rmtree(self.apple)
        self.run_gate(False)

    def test_rejects_mixed_sources_even_with_valid_checksums(self):
        self.rewrite(self.source, "0" * 40)
        self.run_gate(False)

    def test_rejects_unexpected_or_abbreviated_source(self):
        self.run_gate(False, source="0" * 40)
        self.run_gate(False, source=self.source[:7])

    def test_rejects_wrong_platform(self):
        self.rewrite("aarch64-apple-darwin", "aarch64-unknown-linux-gnu")
        self.run_gate(False)

    def test_rejects_duplicate_source(self):
        self.rewrite("source_commit=" + self.source,
                     ("source_commit=" + self.source + "\n") * 2)
        self.run_gate(False)

    def test_rejects_modified_capture(self):
        with (self.apple / "neon.csv").open("a") as stream:
            stream.write("tampered\n")
        self.run_gate(False)

    def test_rejects_weakened_threshold(self):
        self.rewrite("median_minimum_ratio=1.02", "median_minimum_ratio=0.5")
        self.run_gate(False)

    def test_rejects_extra_directory(self):
        (self.apple / "extra").mkdir()
        self.run_gate(False)

    def test_rejects_symlink_and_hardlink(self):
        file = self.apple / "cpu.txt"
        outside = Path(self.temp.name) / "cpu.txt"
        file.rename(outside)
        file.symlink_to(outside)
        self.run_gate(False)
        file.unlink()
        file.hardlink_to(outside)
        self.run_gate(False)

    def test_rejects_fifo_without_blocking(self):
        file = self.apple / "cpu.txt"
        file.unlink()
        os.mkfifo(file)
        self.run_gate(False)

    def test_rejects_directory_symlinks(self):
        alias = Path(self.temp.name) / "alias"
        alias.symlink_to(self.root, target_is_directory=True)
        self.run_gate(False, root=alias)


if __name__ == "__main__":
    unittest.main()
