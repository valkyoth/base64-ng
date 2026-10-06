#!/usr/bin/env python3
"""Check companion feature closure and isolated consumers, never workspace union."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
COMPANIONS = ("bytes", "tokio", "serde", "multibase", "pem", "openpgp")
DEFAULTS = {"bytes": ["std"], "tokio": [], "serde": ["alloc"],
            "multibase": ["alloc", "std"],
            "pem": ["alloc", "std"], "openpgp": ["alloc", "std"]}
EXCLUDED = ("derive", "imap", "mime", "password", "sanitization", "subtle")


def manifest(name):
    return tomllib.loads((ROOT / "crates" / f"base64-ng-{name}" / "Cargo.toml").read_text())


def audit():
    for name in COMPANIONS:
        data = manifest(name)
        features = data["features"]
        assert features["default"] == DEFAULTS[name], name
        assert features["simd"] == ["base64-ng/simd"], name
        assert features["checked-backend"] == ["simd", "base64-ng/checked-backend"], name
        assert data["dependencies"]["base64-ng"]["default-features"] is False, name
        required = {"bytes": ["alloc"], "tokio": ["alloc", "std"],
                    "pem": ["alloc"], "openpgp": ["alloc"]}.get(name, [])
        assert data["dependencies"]["base64-ng"].get("features", []) == required, name
    for name in EXCLUDED:
        assert not {"simd", "checked-backend"} & manifest(name)["features"].keys(), name


def expected_features(name, selected, defaults):
    data = manifest(name)
    pending = list(selected) + (["default"] if defaults else [])
    seen = set()
    core = set(data["dependencies"]["base64-ng"].get("features", []))
    while pending:
        feature = pending.pop()
        if feature in seen:
            continue
        seen.add(feature)
        for child in data["features"][feature]:
            if child.startswith("base64-ng/"):
                core.add(child.split("/", 1)[1])
            elif "/" not in child:
                pending.append(child)
    core_manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    pending = list(core)
    while pending:
        for child in core_manifest["features"][pending.pop()]:
            if child not in core:
                core.add(child)
                pending.append(child)
    return core


def run(toolchain, work, *args, capture=False, coverage=False):
    target = ROOT / "target/companion-features" / toolchain
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target / "build"))
    # Keep the feature matrix reproducible instead of inheriting native/forced ISA flags.
    for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET"):
        env.pop(key, None)
    if coverage:
        env["RUSTFLAGS"] = "-C instrument-coverage"
        env["LLVM_PROFILE_FILE"] = str(work / "coverage-%p-%m.profraw")
    return subprocess.run(["cargo", f"+{toolchain}", *args, "--offline"], cwd=work,
                          env=env, check=True, text=True, capture_output=capture)


def check(toolchain, name, selected, defaults, work, coverage=False, target=None):
    package = f"base64-ng-{name}"
    path = ROOT / "crates" / package
    expected = expected_features(name, selected, defaults)
    smoke = name != "serde" or "alloc" in expected
    # The observer core dependency enables NO features. Only the companion can do so.
    dependencies = f'''companion = {{ package = "{package}", path = {json.dumps(str(path))}, default-features = {str(defaults).lower()}, features = {json.dumps(selected)} }}
base64-ng = {{ path = {json.dumps(str(ROOT))}, default-features = false }}
'''
    extra = {"bytes": 'bytes = { version = "1.12.1", default-features = false }',
             "tokio": 'tokio = { version = "1.53.1", default-features = false, features = ["rt", "io-util"] }',
             "serde": 'serde_json = "1.0.151"'}.get(name, "")
    (work / "Cargo.toml").write_text(f'''[package]
name = "companion-feature-consumer"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[features]
default = {json.dumps(["smoke"] if smoke else [])}
smoke = []
[dependencies]
{dependencies}
[dev-dependencies]
{extra}
''')
    shutil.copyfile(ROOT / "Cargo.lock", work / "Cargo.lock")
    (work / "src").mkdir(exist_ok=True)
    fixture = (ROOT / "portability/companion_features" / f"{name}.rs").read_text()
    common = (ROOT / "portability/companion_features/common.rs").read_text()
    (work / "src/lib.rs").write_text(
        '#![no_std]\n#[cfg(test)]\nextern crate std;\n'
        '#[cfg(all(test, feature = "smoke"))]\nmod tests {\n'
        + common + fixture + '\n}\n')
    print(f"companion features: {toolchain} {name} defaults={defaults} {selected}", flush=True)
    metadata = json.loads(run(toolchain, work, "metadata", "--format-version", "1", capture=True).stdout)
    core_id = next(p["id"] for p in metadata["packages"] if p["name"] == "base64-ng")
    observed = set(next(n["features"] for n in metadata["resolve"]["nodes"] if n["id"] == core_id))
    assert observed == expected, (name, selected, observed, expected)
    # Pin direct resolution to the versions already audited in the root lockfile.
    root_lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    allowed = {(p["name"], p["version"], p.get("source"), p.get("checksum")) for p in root_lock["package"]}
    for p in tomllib.loads((work / "Cargo.lock").read_text())["package"]:
        if "source" in p:
            assert (p["name"], p["version"], p["source"], p.get("checksum")) in allowed, p["name"]
    if target:
        run(toolchain, work, "check", "--locked", "--lib", "--target", target)
    else:
        run(toolchain, work, "test", "--locked", "--release", coverage=coverage)
        run(toolchain, work, "check", "--locked", "--lib")
    if coverage:
        verify_coverage(toolchain, name, work)


def verify_coverage(toolchain, name, work):
    sysroot = subprocess.check_output(["rustc", f"+{toolchain}", "--print", "sysroot"], text=True).strip()
    verbose = subprocess.check_output(["rustc", f"+{toolchain}", "-vV"], text=True)
    host = next(line.removeprefix("host: ") for line in verbose.splitlines() if line.startswith("host: "))
    tools = Path(sysroot) / "lib/rustlib" / host / "bin"
    messages = run(toolchain, work, "test", "--locked", "--release", "--no-run",
                   "--message-format=json", capture=True, coverage=True).stdout
    executables = [m["executable"] for m in map(json.loads, messages.splitlines())
                   if m.get("reason") == "compiler-artifact" and m.get("executable")]
    assert len(executables) == 1, executables
    profile = work / "coverage.profdata"
    subprocess.run([str(tools / "llvm-profdata"), "merge", "-sparse",
                    *map(str, work.glob("*.profraw")), "-o", str(profile)], check=True)
    data = json.loads(subprocess.check_output([str(tools / "llvm-cov"), "export",
        executables[0], f"-instr-profile={profile}"], text=True))
    functions = data["data"][0]["functions"]
    def hit(file, symbol):
        return any(f["count"] > 0 and symbol in f["name"]
                   and any(p.endswith(file) for p in f["filenames"]) for f in functions)
    # Only companion calls transform input in these fixtures. KAT/report calls do
    # not enter either of these ordinary writers or their redundant comparators.
    assert hit("src/encode_backend/checked.rs", "compare_results"), (name, "checked encode not reached")
    assert hit("src/v2/ordinary_decode/vector.rs", "checked"), (name, "checked decode not reached")
    print(f"companion coverage: {name} reached checked ordinary kernels", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--audit-only", action="store_true")
    parser.add_argument("--toolchain", action="append")
    parser.add_argument("--companion", choices=COMPANIONS, action="append")
    parser.add_argument("--coverage", action="store_true",
                        help="require native acceleration and llvm-tools-preview; prove checked kernel execution")
    parser.add_argument("--target", help="compile no_std combinations for this target instead of host tests")
    args = parser.parse_args()
    if args.coverage and args.target:
        parser.error("coverage requires native execution, not --target")
    audit()
    if not args.audit_only:
        active = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        for toolchain in args.toolchain or [active, "1.90.0"]:
            for name in args.companion or COMPANIONS:
                with tempfile.TemporaryDirectory(prefix="base64-companion-") as directory:
                    modes = [(True, []), (False, []), (False, ["simd"]),
                                               (False, ["checked-backend"]),
                                               (True, ["simd"]),
                                               (True, ["checked-backend"]),
                                               (False, ["simd", "std"] if name != "tokio" else ["simd"]),
                                               (False, ["checked-backend", "std"] if name != "tokio" else ["checked-backend"])]
                    for defaults, selected in modes[-1:] if args.coverage else modes:
                        if args.target and "std" in expected_features(name, selected, defaults):
                            continue
                        check(toolchain, name, selected, defaults, Path(directory), args.coverage, args.target)
    print("companion features: ok")


if __name__ == "__main__":
    main()
