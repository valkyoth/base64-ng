#!/usr/bin/env python3
"""Build reviewed revisions and capture native macOS policy comparisons.

Explicit trusted-code operation, NOT an untrusted-candidate sandbox. No network
is needed after cargo fetch. Build caches are private and never reused.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile

if sys.version_info < (3, 12):
    raise SystemExit("Python 3.12+ is required; run this script with python3.12, python3.13 or python3.14.")

import tomllib

from public_api_sandbox import bounded


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trusted-revisions", action="store_true", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--features", nargs="+", choices=("default", "simd", "adapters", "checked"),
                        default=["default", "simd", "adapters", "checked"])
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("native macOS required; use compare-2.1-public-api.py on Linux")
    compare = module("compare", "compare-2.1-public-api.py")
    measure = module("native_measure", "measure-2.1-native-policy.py")
    os.chdir(compare.ROOT)
    if bounded(["git", "status", "--porcelain"]).strip():
        raise ValueError("commit changes before native capture")
    candidate = bounded(["git", "rev-parse", "HEAD"]).decode().strip()
    version = tomllib.loads(Path("rust-toolchain.toml").read_text())["toolchain"]["channel"]
    rustup = shutil.which("rustup")
    if rustup is None:
        raise ValueError("rustup is required")
    compiler = Path(bounded([rustup, "which", "--toolchain", version, "rustc"],
                            env=dict(PATH=os.defpath, HOME=str(Path.home()))).decode().strip())
    tools = compiler.parent
    files = [*sorted((compare.ROOT / "perf/public-api/src").rglob("*.rs")),
             *[compare.ROOT / name for name in ("perf/public-api/Cargo.toml", "perf/public-api/Cargo.lock",
                "perf/src/allocation.rs", "src/v2/rfc4648_oracle.rs", "scripts/public_api_policy.py",
                "scripts/public_api_baseline.py", "scripts/public_api_sandbox.py", "scripts/compare-2.1-public-api.py",
                "scripts/measure-2.1-native-policy.py", "scripts/capture-2.1-macos-policy.py")]]
    hashes = {str(p.relative_to(compare.ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    destination = args.output.resolve()
    destination.mkdir(mode=0o700, parents=True, exist_ok=False)
    manifest = dict(schema=1, status="running", scope=__doc__, baseline=compare.BASELINE, candidate=candidate,
                    harness_sha256=hashes, features=args.features, binaries={}, captures={}, locks_sha256={},
                    rustflags="--cfg base64_ng_perf_evidence",
                    compiler=bounded([str(compiler), "-Vv"]).decode(),
                    tool_sha256={name: hashlib.sha256((tools / name).read_bytes()).hexdigest()
                                 for name in ("rustc", "cargo")})
    try:
        with tempfile.TemporaryDirectory(prefix="base64-ng-policy-") as temporary:
            private = Path(temporary)
            cargo_home = private / "cargo"
            cargo_home.mkdir()
            (cargo_home / "registry").symlink_to(Path.home() / ".cargo/registry", target_is_directory=True)
            env = dict(PATH=str(tools) + os.pathsep + os.defpath, HOME=str(private), CARGO_HOME=str(cargo_home),
                       LC_ALL="C", CARGO_INCREMENTAL="0", RUSTFLAGS="--cfg base64_ng_perf_evidence")
            trees = {}
            for side, revision in (("baseline", compare.BASELINE), ("candidate", candidate)):
                tree = private / side
                tree.mkdir()
                compare.extract_revision(revision, tree)
                compare.install_harness(tree, files)
                lock = destination / f"{side}-Cargo.lock"
                shutil.copyfile(compare.prepare_lock(tree), lock)
                manifest["locks_sha256"][lock.name] = hashlib.sha256(lock.read_bytes()).hexdigest()
                trees[side] = tree
            for feature in args.features:
                binaries = {}
                sides = ["baseline", "candidate"] + ([] if feature == "default" else ["controls"])
                for side in sides:
                    tree = trees["candidate" if side == "controls" else side]
                    target = private / f"build-{feature}-{side}"
                    features = compare.FEATURES[feature] + (",validation-policy" if side == "controls" else "")
                    bounded([str(tools / "cargo"), "build", "--offline", "--locked", "--release", "--no-default-features",
                             "--manifest-path", str(tree / "perf/public-api/Cargo.toml"), "--target-dir", str(target),
                             "--features", features], env=env, timeout=600, stderr_limit=4 * 1024 * 1024)
                    binaries[side] = target / "release/base64-ng-public-api-perf"
                    manifest["binaries"][f"{feature}/{side}"] = hashlib.sha256(binaries[side].read_bytes()).hexdigest()
                for label in ["revisions"] + ([] if feature == "default" else ["controls"]):
                    print(f"native policy: {feature}/{label}", flush=True)
                    controls = label == "controls"
                    report = measure.capture(binaries["controls" if controls else "baseline"],
                                             binaries["controls" if controls else "candidate"], controls=controls)
                    path = destination / f"{feature}-{label}.json"
                    path.write_text(json.dumps(report, indent=2) + "\n")
                    manifest["captures"][path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
            if any(hashlib.sha256(p.read_bytes()).hexdigest() != hashes[str(p.relative_to(compare.ROOT))] for p in files):
                raise ValueError("harness changed during capture")
        manifest["status"] = "passed"
    except BaseException:
        manifest["status"] = "failed"
        raise
    finally:
        (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
