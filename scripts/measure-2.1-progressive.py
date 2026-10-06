#!/usr/bin/env python3
"""Paired native measurements of trusted test-only progressive prototype code."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
SIZES = (4096, 16384, 65536, 1048576)
TEST = "v2::ordinary_decode::progressive_candidate::benchmark::paired_complete_operations"


def redact_build_log(text):
    # The checkout can be inside the home directory: replace it first.
    return text.replace(str(ROOT), "<REPOSITORY>").replace(str(Path.home()), "<HOME>")


def write_log(path, text):
    path.write_text(redact_build_log(text), encoding="utf-8")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse(text):
    samples, backends = {}, {}
    for line in text.splitlines():
        if line.startswith("progressive-backend,"):
            _, profile, size, backend = line.split(",")
            key = (int(profile), int(size))
            if key in backends or backend not in {"avx2", "ssse3-sse4.1"}:
                raise ValueError("duplicate or unexpected native backend")
            backends[key] = backend
        elif line.startswith("progressive-bench,"):
            _, profile, size, sample, mode, rounds, nanos = line.split(",")
            key = tuple(map(int, (profile, size, sample, mode)))
            if key in samples or int(rounds) != max(4, 4194304 // int(size)) or int(nanos) <= 0:
                raise ValueError("duplicate or invalid timing sample")
            samples[key] = int(nanos)
    expected = {(p, n, s, m) for p in range(4) for n in SIZES for s in range(11) for m in range(3)}
    if set(samples) != expected or set(backends) != {(p, n) for p in range(4) for n in SIZES}:
        raise ValueError("incomplete paired samples or backend observations")
    rows = []
    for profile in range(4):
        for size in SIZES:
            paired = [samples[profile, size, s, 0] / samples[profile, size, s, 1] for s in range(11)]
            rows.append({"profile": profile, "decoded_bytes": size,
                         "backend": backends[profile, size],
                         "transactional_over_progressive": paired,
                         "median_speedup": statistics.median(paired),
                         "median_ns_per_call": {
                             mode: statistics.median(samples[profile, size, s, m] for s in range(11))
                                   / max(4, 4194304 // size)
                             for m, mode in enumerate(("transactional", "progressive", "incremental"))}})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=3, choices=range(1, 6))
    args = parser.parse_args()
    if platform.machine() != "x86_64":
        parser.error("this private experiment requires native x86_64")
    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target/progressive-measure" / toolchain))
    for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET", "CARGO_BUILD_BUILD_DIR"):
        env.pop(key, None)
    files = sorted(set([*ROOT.joinpath("src").rglob("*.rs"), ROOT / "Cargo.toml", ROOT / "Cargo.lock",
                        ROOT / "rust-toolchain.toml", Path(__file__).resolve()]))
    digests = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    record = {"schema": 1, "purpose": "private progressive go/no-go, not release admission",
              "log_redaction": {"policy": "repository-home-v1", "applied": "during-capture"},
              "host": platform.node(), "platform": platform.platform(),
              "cpu": next(line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines() if line.startswith("model name")),
              "rustc": subprocess.check_output(["rustc", f"+{toolchain}", "-Vv"], text=True),
              "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "source_sha256": digests, "runs": []}
    args.output.mkdir(parents=True, exist_ok=False)
    for features in ("std,simd", "std,checked-backend"):
        for repetition in range(args.runs):
            command = ["cargo", f"+{toolchain}", "test", "--locked", "--offline", "--release", "--no-default-features",
                       "--features", features, "--lib", TEST, "--", "--ignored", "--exact", "--nocapture", "--test-threads=1"]
            result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, timeout=240, check=True)
            # One test thread still prefixes the first println with its test name.
            stdout = result.stdout.replace(f"test {TEST} ... ", "")
            rows = parse(stdout)
            name = f"{'checked' if 'checked' in features else 'plain'}-{repetition}.log"
            log_sha256 = write_log(args.output / name, result.stderr + stdout)
            record["runs"].append({"features": features, "repetition": repetition, "command": command,
                                   "log": name, "log_sha256": log_sha256, "rows": rows})
            print(f"progressive measurement: {features} repetition={repetition} median speedups="
                  + ",".join(f"{row['median_speedup']:.3f}" for row in rows), flush=True)
    if any(hashlib.sha256((ROOT / name).read_bytes()).hexdigest() != digest for name, digest in digests.items()):
        raise RuntimeError("measured sources changed during capture")
    (args.output / "summary.json").write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    main()
