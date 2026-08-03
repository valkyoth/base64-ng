#!/usr/bin/env python3
"""Prove that protocol registry source, corpus, name, and model drift fail closed."""

from __future__ import annotations

import pathlib
import shutil
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts/validate-protocol-registry.py"


def replace(path: pathlib.Path, old: bytes, new: bytes) -> None:
    data = path.read_bytes()
    if old not in data:
        raise AssertionError(f"mutation anchor missing in {path}")
    path.write_bytes(data.replace(old, new, 1))


def expect_failure(name: str, mutation) -> None:
    with tempfile.TemporaryDirectory(prefix="base64-ng-protocol-registry-") as temporary:
        tree = pathlib.Path(temporary) / "repo"
        shutil.copytree(ROOT, tree, ignore=shutil.ignore_patterns("target", ".git"))
        mutation(tree)
        result = subprocess.run(
            ["python3", str(VALIDATOR), "--root", str(tree)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            raise SystemExit(f"protocol registry mutation unexpectedly passed: {name}")


expect_failure(
    "missing protocol claim",
    lambda root: replace(root / "protocol-registry/v1/protocols.tsv", b"mime-body\t", b"mime-body-removed\t"),
)
expect_failure(
    "public name drift",
    lambda root: replace(root / "src/v2/profiles.rs", b"MIME_BODY_STRICT", b"MIME_BODY_CHANGED"),
)
expect_failure(
    "corpus decision drift",
    lambda root: replace(root / "protocol-registry/v1/cases.tsv", b"mime-f\tmime-body\taccept", b"mime-f\tmime-body\treject"),
)
expect_failure(
    "source bytes drift",
    lambda root: replace(root / "rfc/rfc4648.txt", b"Base 64 Encoding", b"Base 64 EncodinG"),
)
expect_failure(
    "independent model imports production",
    lambda root: (root / "protocol-registry/runner/src/model.rs").write_text(
        (root / "protocol-registry/runner/src/model.rs").read_text(encoding="utf-8")
        + "\n// base64_ng forbidden dependency\n",
        encoding="utf-8",
    ),
)

print("protocol registry mutation tests: ok")
