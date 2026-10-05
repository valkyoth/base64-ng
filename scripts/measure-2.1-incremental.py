#!/usr/bin/env python3
"""Paired trusted incremental/adapter benchmarks, including setup and finish.

Build both existing public-api harnesses on the same Rust with std,simd and
base64_ng_perf_evidence; add adapters for sync/bytes/tokio operations. Copy the
same harness to both builds. These unsandboxed timings are not release admission.
"""
import argparse
import csv
import hashlib
import io
import itertools
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
    parser.add_argument("--operations", nargs="+", choices=("incremental", "sync", "bytes", "tokio"),
                        default=["incremental"])
    args = parser.parse_args()
    binaries = {side: getattr(args, side).resolve() for side in ("baseline", "candidate")}
    hashes = {side: hashlib.sha256(path.read_bytes()).hexdigest() for side, path in binaries.items()}
    rows = []
    for operation, direction in itertools.product(args.operations, ("encode", "decode")):
        for profile in ("sp", "su", "up", "uu"):
            for size in (32, 768, 65536, 1048576):
                rounds = max(1, min(10000, 262144 // size))
                encoded = (size + 2) // 3 * 4 if profile.endswith("p") else (size * 8 + 5) // 6
                for fragment in (7, 256, 4096, 1048576):
                    case = [operation, direction, profile, size, "random", fragment, "warm"]
                    for sample in range(7):
                        for side in (("baseline", "candidate") if sample % 2 == 0 else ("candidate", "baseline")):
                            command = [str(binaries[side]), operation, direction, profile,
                                       str(size), "random", str(fragment), str(rounds), "warm"]
                            output = subprocess.check_output(command, text=True, timeout=120)
                            parsed = list(csv.DictReader(io.StringIO(output)))
                            if len(parsed) != 1:
                                raise ValueError("expected one measurement row")
                            row = parse_sample(parsed[0], rounds, size, encoded)
                            if row["allocations"] != 0:
                                raise ValueError("bounded processing allocated")
                            features = "std,simd" if args.operations == ["incremental"] else "std,simd,adapters"
                            rows.append(dict(row, case=case, sample=sample, side=side, features=features))
                    print(f"measured {operation}/{direction}/{profile}/{size}/{fragment}", file=sys.stderr, flush=True)
    if any(hashlib.sha256(path.read_bytes()).hexdigest() != hashes[side] for side, path in binaries.items()):
        raise ValueError("benchmark executable changed during capture")
    print(json.dumps(dict(scope="trusted incremental/adapter development comparison, not release admission",
                          host=platform.platform(), binary_sha256=hashes,
                          rows=rows, summary=summarize(rows)), indent=2))


if __name__ == "__main__":
    main()
