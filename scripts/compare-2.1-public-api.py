#!/usr/bin/env python3
"""Short paired public-API measurements; not a release-admission campaign."""

import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import shutil
import tarfile
import tempfile
import tomllib

from public_api_baseline import parse_sample, summarize
from public_api_sandbox import Sandbox, bounded, BUILD_ENV, RUNTIME_ENV
from public_api_sandbox import aggregate_limits, tool_inventory

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "816da2e1e4a66c913057c86d149068f1c88776bf"
FEATURES = {"core": "", "alloc": "alloc", "default": "std", "simd": "std,simd",
            "checked": "std,simd,checked", "adapters": "adapters,simd"}


def install_harness(tree, files):
    destination = tree / "perf/public-api"
    if destination.exists():
        shutil.rmtree(destination)
    for source in files:
        relative = source.relative_to(ROOT)
        if relative.parts[:2] == ("perf", "public-api") and source.name != "Cargo.lock":
            target = tree / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
    for source in ["perf/src/allocation.rs", "src/v2/rfc4648_oracle.rs"]:
        shutil.copyfile(ROOT / source, tree / source)


def output(command, **kwargs):
    return bounded(command, **kwargs).decode("utf-8")


def extract_revision(revision, tree):
    archive = bounded(["/usr/bin/git", "archive", "--format=tar", revision], limit=128 * 1024 * 1024)
    with tarfile.open(fileobj=io.BytesIO(archive)) as source:
        members = []
        for member in source:
            if len(members) >= 10000:
                raise ValueError("source archive exceeds member count limit")
            members.append(member)
        if sum(member.size for member in members) > 128 * 1024 * 1024:
            raise ValueError("source archive exceeds size limit")
        for member in members:
            path = Path(member.name)
            if path.is_absolute() or ".." in path.parts or not (member.isfile() or member.isdir()):
                raise ValueError("source archive contains a link or unsafe path/type")
        source.extractall(tree, members=members, filter="data")


def prepare_lock(tree):
    reference = tomllib.loads((ROOT / "perf/public-api/Cargo.lock").read_text())
    for package in reference["package"]:
        if "source" in package or package["name"] == "base64-ng-public-api-perf":
            continue
        name = package["name"]
        if name not in {"base64-ng", "base64-ng-bytes", "base64-ng-tokio"}:
            raise ValueError("unexpected local harness dependency")
        manifest = tree / ("Cargo.toml" if name == "base64-ng" else f"crates/{name}/Cargo.toml")
        package["version"] = tomllib.loads(manifest.read_text())["package"]["version"]
    lines = [f'version = {reference["version"]}']
    for package in reference["package"]:
        lines.append("\n[[package]]")
        lines.extend(f"{key} = {json.dumps(value)}" for key, value in package.items())
    lock = tree / "perf/public-api/Cargo.lock"
    lock.write_text("\n".join(lines) + "\n")
    return lock


def cases(names, smoke, full):
    sizes = [0, 1, 2, 3, 32, 512, 4096, 65536]
    if full:
        sizes += [15, 16, 17, 31, 33, 47, 48, 49, 63, 64, 65, 1048576]
    if smoke:
        sizes = [0, 3, 32, 512]
    for name in names:
        for profile in (["sp"] if smoke else ["sp", "su", "up", "uu"]):
            for size in sizes:
                for pattern in (["random", "structured", "zero"] if full else ["random"]):
                    for direction in ["encode", "decode"]:
                        if name == "validate" and direction == "encode":
                            continue
                        yield [name, direction, profile, str(size), pattern, "4096", "warm"]
                if size in [32, 65536]:
                    for direction in ["encode", "decode"]:
                        if name != "validate" or direction == "decode":
                            yield [name, direction, profile, str(size), "random", "4096", "cold"]
                if size in [512, 65536]:
                    if name in ["canonical", "historical", "validate"]:
                        encoded_len = (size + 2) // 3 * 4 if profile.endswith("p") else (size * 8 + 5) // 6
                        for position in [0, 31, 32, 63, 64, encoded_len - 1]:
                            yield [name, "decode", profile, str(size), "random", "4096", f"invalid-{position}"]
                    if size == 512 and name in ["incremental", "sync", "bytes", "tokio"]:
                        for fragment in ["1", "7"]:
                            for direction in ["encode", "decode"]:
                                yield [name, direction, profile, str(size), "structured", fragment, "warm"]


def capture(args):
    os.chdir(ROOT)
    dirty = output(["git", "status", "--porcelain"]).strip()
    if dirty and not args.allow_dirty_harness:
        raise ValueError("commit changes first, or use --allow-dirty-harness for a diagnostic run")
    candidate = output(["git", "rev-parse", f"{args.candidate}^{{commit}}"]).strip()
    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    sandbox = Sandbox(toolchain)
    destination = Path(args.output).resolve()
    destination.mkdir(parents=True, exist_ok=False)
    source_files = [*sorted((ROOT / "perf/public-api/src").rglob("*.rs")), ROOT / "perf/public-api/Cargo.toml",
                    ROOT / "perf/public-api/Cargo.lock", ROOT / "perf/src/allocation.rs",
                    ROOT / "src/v2/rfc4648_oracle.rs", Path(__file__), ROOT / "scripts/public_api_baseline.py",
                    ROOT / "scripts/public_api_sandbox.py", ROOT / "scripts/public_api_cgroup.py"]
    digests = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in source_files}
    manifest = dict(schema=1, baseline=BASELINE, candidate=candidate, harness_commit=output(["git", "rev-parse", "HEAD"]).strip(),
                    diagnostic_dirty_harness=bool(dirty), harness_sha256=digests,
                    rustc=sandbox.execute(["/toolchain/bin/rustc", "-Vv"], build=True).decode(),
                    cargo=sandbox.execute(["/toolchain/bin/cargo", "-V"], build=True).decode(), python=platform.python_version(), platform=platform.platform(),
                    machine=platform.machine(), features=args.features, samples=args.samples,
                    command=list(os.sys.argv), scope="exploratory paired baseline, not admission",
                    binaries={}, build_environment=BUILD_ENV, runtime_environment=RUNTIME_ENV,
                    sandbox="bubblewrap: no network, no host home, readonly source; quota-limited tmpfs",
                    aggregate_limits=aggregate_limits(),
                    tool_paths={"toolchain": str(sandbox.toolchain), "registry": str(sandbox.registry)},
                    tool_sha256={str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                                 for p in tool_inventory(sandbox.toolchain)})
    if Path("/proc/cpuinfo").exists():
        manifest["cpuinfo"] = Path("/proc/cpuinfo").read_text()
    else:
        manifest["processor"] = platform.processor()
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    rows = []
    with tempfile.TemporaryDirectory(prefix="base64-ng-paired-") as temporary:
        trees = {}
        for side, revision in [("baseline", BASELINE), ("candidate", candidate)]:
            tree = Path(temporary) / side
            tree.mkdir()
            extract_revision(revision, tree)
            install_harness(tree, source_files)
            shutil.copyfile(prepare_lock(tree), destination / f"{side}-Cargo.lock")
            trees[side] = tree
        for feature in args.features:
            binaries = {}
            available = {}
            for side, tree in trees.items():
                binary = Path(temporary) / f"{feature}-{side}-benchmark"
                binary.write_bytes(sandbox.compile(tree, FEATURES[feature]))
                binary.chmod(0o700)
                binaries[side] = binary
                available[side] = sandbox.execute(["/benchmark", "list"], binary=binary).decode().splitlines()
                manifest["binaries"][f"{feature}-{side}"] = dict(
                    sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), operations=available[side])
                (destination / f"{feature}-{side}-diagnostics.txt").write_bytes(
                    sandbox.execute(["/benchmark", "diagnostics"], binary=binary, limit=8 * 1024 * 1024))
            if available["baseline"] != available["candidate"]:
                raise ValueError("backend/operation availability changed; review before comparison")
            if (destination / f"{feature}-baseline-diagnostics.txt").read_bytes() != (destination / f"{feature}-candidate-diagnostics.txt").read_bytes():
                raise ValueError("diagnostic or destination-mutation regression")
            names = available["baseline"]
            if args.smoke:
                names = [n for n in names if n in ["canonical", "historical", "validate", "incremental", "sync", "bytes", "tokio"]]
            print(f"paired baseline: {feature}; {len(names)} operations", flush=True)
            with (destination / "samples.jsonl").open("a") as log:
                for case in cases(names, args.smoke, args.full):
                    size = int(case[3])
                    encoded = (size + 2) // 3 * 4 if case[2].endswith("p") else (size * 8 + 5) // 6
                    iterations = 1 if case[-1] == "cold" else min(512, max(1, 16384 // max(size, 32)))
                    for sample in range(args.samples):
                        for side in (["baseline", "candidate"] if sample % 2 == 0 else ["candidate", "baseline"]):
                            raw = sandbox.execute(["/benchmark", *case[:-1], str(iterations), case[-1]], binary=binaries[side]).decode()
                            parsed = list(csv.DictReader(io.StringIO(raw)))
                            if len(parsed) != 1:
                                raise ValueError("missing measurement")
                            row = parse_sample(parsed[0], iterations, size, encoded)
                            row.update(side=side, sample=sample, features=feature, case=case)
                            rows.append(row)
                            log.write(json.dumps(row) + "\n")
                            log.flush()
    if any(hashlib.sha256(p.read_bytes()).hexdigest() != digests[str(p.relative_to(ROOT))] for p in source_files):
        raise ValueError("harness changed during capture")
    summary = summarize(rows)
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (destination / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(destination.iterdir()) if p.is_file()}
    (destination / "SHA256SUMS.json").write_text(json.dumps(hashes, indent=2) + "\n")
    counts = {status: sum(row["status"] == status for row in summary) for status in sorted({row["status"] for row in summary})}
    print(f"paired baseline: complete: {counts}; {destination}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", default="HEAD")
    parser.add_argument("--output", required=True)
    parser.add_argument("--features", nargs="+", choices=FEATURES, default=["default", "adapters"])
    parser.add_argument("--samples", type=int, default=7)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--smoke", action="store_true")
    modes.add_argument("--full", action="store_true")
    parser.add_argument("--allow-dirty-harness", action="store_true")
    args = parser.parse_args()
    if not 1 <= args.samples <= 100:
        parser.error("samples must be 1..100")
    capture(args)


if __name__ == "__main__":
    main()
