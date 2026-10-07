#!/usr/bin/env python3
"""Offline publisher tests: no credentials or registry publication required."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = Path("packages/base64-ng-wasm-loader")


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="wasm-release-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "scripts").mkdir()
        (self.root / "bin").mkdir()
        (self.root / PACKAGE).mkdir(parents=True)
        shutil.copy2(ROOT / PACKAGE / "package.json", self.root / PACKAGE / "package.json")
        shutil.copy2(ROOT / "scripts/release_wasm_loader.sh", self.root / "scripts/release_wasm_loader.sh")
        (self.root / "Cargo.toml").write_text('[package]\nversion = "2.1.0"\n')
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("BASE64_NG_")}
        self.env.update(PATH=f"{self.root / 'bin'}:{os.environ['PATH']}", CASE="ok")
        self.command("scripts/release_crates.py", '''
printf 'release=2.1.0\nname=@valkyoth/base64-ng-wasm-loader\nversion=2.1.0\npublish=true\n'
''')
        self.command("bin/git", '''
case "$1" in
  status)
    if [ "$CASE" = dirty ] || { [ "$CASE" = dirty-after ] && [ -f built ]; }; then
      echo ' M src/lib.rs'
    fi;;
  rev-parse)
    if [ "$CASE" = head-after ] && [ -f built ]; then echo changed; else echo reviewed; fi;;
  rev-list)
    if [ "$CASE" = tag ] || { [ "$CASE" = tag-after ] && [ -f built ]; }; then
      echo different
    else echo reviewed; fi;;
  *) exit 99;;
esac
''')
        self.command("scripts/verify-release-tag.sh", '''
echo verified >> events
if [ "$CASE" = signature ] || { [ "$CASE" = signature-after ] && [ -f built ]; }; then
  exit 1
fi
if [ "$CASE" = artifact-after ] && [ -f built ]; then
  printf altered >"$BASE64_NG_WASM_INSTALL_DIR/packed/valkyoth-base64-ng-wasm-loader-2.1.0.tgz"
fi
''')
        self.command("scripts/check-2.0-wasm-loader.sh", '''
echo gate >> events
touch built
python3 - <<'PY'
import os, stat
from pathlib import Path
p = Path(os.environ["BASE64_NG_WASM_INSTALL_DIR"])
if stat.S_IMODE(p.stat().st_mode) != 0o700:
    raise SystemExit("not private")
Path("private-dir").write_text(str(p))
PY
[ "$CASE" != gate-failure ] || exit 1
mkdir -p "$BASE64_NG_WASM_INSTALL_DIR/packed"
cd "$BASE64_NG_WASM_INSTALL_DIR/packed"
printf 'exact tested bytes' >valkyoth-base64-ng-wasm-loader-2.1.0.tgz
sha256sum valkyoth-base64-ng-wasm-loader-2.1.0.tgz >checked.sha256
[ "$CASE" != missing-receipt ] || rm checked.sha256
''')
        self.command("bin/npm", '''
python3 - "$@" <<'PY'
import json, sys
from pathlib import Path
args = sys.argv[1:]
if args[0] != "publish" or "--ignore-scripts" not in args:
    raise SystemExit("unexpected npm invocation")
p = Path(args[-1])
if p.read_bytes() != b"exact tested bytes":
    raise SystemExit("not tested bytes")
Path("published.json").write_text(json.dumps(args))
PY
''')

    def command(self, name, body):
        path = self.root / name
        path.write_text("#!/bin/sh\nset -eu\n" + body)
        path.chmod(0o700)

    def run_release(self, mode, case="ok"):
        return subprocess.run(
            ["sh", "scripts/release_wasm_loader.sh", mode], cwd=self.root,
            env=dict(self.env, CASE=case), capture_output=True, text=True, timeout=30,
        )

    def test_exact_archive_and_modes(self):
        directories = set()
        for mode, flag in [("publish", "--provenance"),
                           ("publish-desktop", "--provenance=false"),
                           ("dry-run", "--dry-run")]:
            with self.subTest(mode=mode):
                result = self.run_release(mode)
                self.assertEqual(result.returncode, 0, result.stderr)
                args = json.loads((self.root / "published.json").read_text())
                self.assertIn(flag, args)
                self.assertIn("--access", args)
                self.assertFalse(Path(args[-1]).exists(), "private artifacts leaked")
                directories.add(Path(args[-1]).parent.parent)
        self.assertEqual(len(directories), 3, "release directory was reused")
        self.assertEqual((self.root / "events").read_text().splitlines(),
                         ["verified", "gate", "verified"] * 3)

    def test_rejects_source_tag_and_artifact_changes(self):
        for case in ("dirty", "tag", "signature", "dirty-after", "head-after",
                     "tag-after", "signature-after", "artifact-after",
                     "gate-failure", "missing-receipt"):
            with self.subTest(case=case):
                (self.root / "built").unlink(missing_ok=True)
                result = self.run_release("publish-desktop", case)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertFalse((self.root / "published.json").exists())
                self.assertEqual(list((self.root / "target").glob("npm-release.*")), [])
                if case in ("dirty", "tag", "signature"):
                    self.assertFalse((self.root / "built").exists())

    def test_raw_directory_publish_is_blocked_by_real_npm(self):
        package = self.root / PACKAGE
        (package / "scripts").mkdir()
        shutil.copy2(ROOT / PACKAGE / "scripts/reject-directory-publish.mjs",
                     package / "scripts/reject-directory-publish.mjs")
        # --dry-run and loopback registry guarantee this fixture cannot upload.
        result = subprocess.run(
            [shutil.which("npm"), "publish", "--dry-run", "--ignore-scripts=false",
             "--registry=http://127.0.0.1:9", f"--cache={self.root / 'cache'}"],
            cwd=package, capture_output=True, text=True, timeout=30,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Direct npm publish is disabled", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
