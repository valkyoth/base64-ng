#!/usr/bin/env python3
"""Check publish inventories without publishing unpublished workspace dependencies."""

from pathlib import PurePosixPath
import subprocess

from release_crates import DEFAULT_PLAN, ROOT, release_plan


REPOSITORY_ONLY = {
    ".git", ".github", "api-snapshots", "benches", "docs", "fuzz",
    "hardware-evidence", "kani", "perf", "portability", "release-notes",
    "scripts", "security", "target", "rfc", "spec",
}


def validate_inventory(package, files):
    if not files or len(files) != len(set(files)):
        raise ValueError(f"{package}: empty or duplicate inventory")
    for name in files:
        path = PurePosixPath(name)
        if not path.parts or path.is_absolute() or ".." in path.parts or "\\" in name:
            raise ValueError(f"{package}: unsafe package path {name}")
        if path.parts[0] in REPOSITORY_ONLY:
            raise ValueError(f"{package}: repository-only payload {name}")
    required = {"Cargo.toml", "README.md", "src/lib.rs"}
    if package == "base64-ng":
        required |= {"examples/decode_policy.rs", "examples/stream_file.rs",
                     "LICENSE-MIT", "LICENSE-APACHE"}
    if not required <= set(files):
        raise ValueError(f"{package}: missing {sorted(required - set(files))}")
    if package == "base64-ng" and len(files) > 238:
        raise ValueError("core package exceeds the reviewed 238-file ceiling")


def main():
    plan = release_plan(DEFAULT_PLAN)
    if plan["version"] != "2.1.0":
        raise SystemExit("2.1 package freeze requires release version 2.1.0")
    output = ROOT / "target/release-evidence/2.1-freeze/package-lists"
    output.mkdir(parents=True, exist_ok=True)
    for package in plan["crates"]:
        snapshot = ROOT / "api-snapshots/v2.1.0" / f"{package}.txt"
        if not snapshot.is_file() or not snapshot.stat().st_size:
            raise SystemExit(f"missing API snapshot: {package}")
        result = subprocess.run(
            ["cargo", "package", "--locked", "--allow-dirty", "--list", "-p", package],
            cwd=ROOT, check=True, capture_output=True, text=True,
        )
        files = result.stdout.splitlines()
        validate_inventory(package, files)
        (output / f"{package}.txt").write_text(result.stdout, encoding="utf-8")
        print(f"{package}: {len(files)} package files")


if __name__ == "__main__":
    main()
