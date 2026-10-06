#!/usr/bin/env python3
"""Compile-check the unpublished ABI probe's unsafe call boundaries."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def check(compiler):
    with tempfile.TemporaryDirectory(prefix="windows-abi-contract-") as directory:
        work = Path(directory)
        (work / "src").mkdir()
        (work / "Cargo.toml").write_text(
            '[package]\nname = "windows-abi-contract"\nversion = "0.0.0"\n'
            'edition = "2024"\n[workspace]\n[dependencies]\n'
            f'base64-ng = {{ path = {json.dumps(ROOT.as_posix(), ensure_ascii=False)}, '
            'default-features = false, features = ["std", "simd"] }\n',
            encoding="utf-8",
        )
        fixture = json.dumps((ROOT / "portability/windows_native/src/abi.rs").as_posix(),
                             ensure_ascii=False)
        source = f'#![allow(dead_code)]\ninclude!({fixture});\n' + '''
fn main() {
    let mut context = Context {
        input: &[], encoded: &[], output: &mut [], operation: 0, token: None,
    };
    BODY
}
'''
        env = dict(os.environ, CARGO_TARGET_DIR=str(work / "target"),
                   CARGO_BUILD_BUILD_DIR=str(work / "build"), CARGO_INCREMENTAL="0")
        for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER",
                    "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET"):
            env.pop(key, None)
        cases = (
            ("unsafe { invoke(core::ptr::from_mut(&mut context)); call(&mut context, invoke); }", 0),
            ("invoke(core::ptr::from_mut(&mut context));", 1),
            ("call(&mut context, invoke);", 1),
        )
        for body, expected_errors in cases:
            (work / "src/main.rs").write_text(source.replace("BODY", body), encoding="utf-8")
            result = subprocess.run(
                ["cargo", f"+{compiler}", "check", "--offline", "--message-format=json"],
                cwd=work, env=env, capture_output=True, text=True, timeout=120,
            )
            messages = [json.loads(line) for line in result.stdout.splitlines()]
            errors = [m["message"] for m in messages if m.get("reason") == "compiler-message"
                      and m["message"]["level"] == "error"]
            if ((result.returncode == 0) != (expected_errors == 0)
                    or len(errors) != expected_errors
                    or any((error.get("code") or {}).get("code") != "E0133" for error in errors)):
                raise RuntimeError(f"Unexpected contract check result: {errors}\n{result.stderr}")
        print(f"Windows ABI contract: {compiler}: unsafe calls accepted; both safe calls rejected")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", action="append")
    args = parser.parse_args()
    active = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    for compiler in args.toolchain or [active]:
        check(compiler)
