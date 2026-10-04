#!/usr/bin/env python3
"""Compare two trusted frozen public-api harness builds, not untrusted code.

Both must use the same compiler and std,simd features plus base64_ng_perf_evidence.
This unsandboxed development capture does not replace release admission.
"""
import argparse
import csv
import hashlib
import io
import json
import platform
import subprocess
import sys
from pathlib import Path

from public_api_baseline import parse_sample, summarize


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--wasmtime", type=Path, help="execute trusted WASI harnesses")
    parser.add_argument("--direction", choices=["encode", "decode"], default="encode",
                        help="complete public call direction (default: encode)")
    args = parser.parse_args()
    binaries = {side: getattr(args, side).resolve() for side in ["baseline", "candidate"]}
    hashes = {side: hashlib.sha256(path.read_bytes()).hexdigest() for side, path in binaries.items()}
    prefix = [str(args.wasmtime.resolve()), "run", "-C", "cache=n"] if args.wasmtime else []
    rows = []
    operations = ["canonical", "historical"] if prefix else ["canonical", "historical", "owned", "append"]
    if args.direction == "decode" and not prefix:
        operations += ["historical-owned", "string-owned"]
    sizes = [32, 65536] if prefix else [0, 3, 32, 192, 768, 65536, 1048576]
    for operation in operations:
        for profile in ["sp", "su", "up", "uu"]:
            for size in sizes:
                rounds = 10000 if size <= 768 else 100
                encoded = (size + 2) // 3 * 4 if profile.endswith("p") else (size * 8 + 5) // 6
                case = [operation, args.direction, profile, size, "random", 4096, "warm"]
                for sample in range(7):
                    for side in (["baseline", "candidate"] if sample % 2 == 0 else ["candidate", "baseline"]):
                        command = prefix + [str(binaries[side]), operation, args.direction, profile,
                                            str(size), "random", "4096", str(rounds), "warm"]
                        parsed = list(csv.DictReader(io.StringIO(subprocess.check_output(command, text=True, timeout=120))))
                        if len(parsed) != 1:
                            raise ValueError("expected one measurement row")
                        row = parse_sample(parsed[0], rounds, size, encoded)
                        if operation not in {"owned", "historical-owned", "string-owned"} and row["allocations"] != 0:
                            raise ValueError("unexpected timed allocation")
                        rows.append(dict(row, case=case, sample=sample, side=side, features="std,simd"))
                print(f"measured {operation}/{profile}/{size}", file=sys.stderr, flush=True)
    if any(hashlib.sha256(path.read_bytes()).hexdigest() != hashes[side] for side, path in binaries.items()):
        raise ValueError("benchmark binary changed during measurement")
    print(json.dumps(dict(scope="trusted development comparison, not release admission",
                          host=platform.platform(), runner=prefix, binary_sha256=hashes,
                          rows=rows, summary=summarize(rows)), indent=2))


if __name__ == "__main__":
    main()
