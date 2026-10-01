#!/usr/bin/env python3
"""Paired development measurements of two trusted, locally built harnesses.

Use perf/public-api with std,simd and base64_ng_perf_evidence, identical compiler
and flags on both revisions. This executes the supplied binaries unsandboxed;
it is not the untrusted-repository comparison runner or a release admission.
"""
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
    if len(sys.argv) != 3 or platform.machine() != "riscv64":
        sys.exit("usage (native RISC-V only): measure-2.1-rvv-public-api.py BASELINE CANDIDATE")
    binaries = dict(zip(["baseline", "candidate"], [Path(p).resolve() for p in sys.argv[1:]]))
    hashes = {side: hashlib.sha256(path.read_bytes()).hexdigest() for side, path in binaries.items()}
    rows = []
    for operation in ["canonical", "historical"]:
        for profile in ["sp", "su", "up", "uu"]:
            for size in [0, 3, 32, 768, 1024, 65536, 1048576]:
                rounds = 10000 if size <= 32 else 100 if size <= 1024 else 10
                encoded = (size + 2) // 3 * 4 if profile.endswith("p") else (size * 8 + 5) // 6
                case = [operation, "decode", profile, size, "random", 4096, "warm"]
                for sample in range(7):
                    for side in (["baseline", "candidate"] if sample % 2 == 0 else ["candidate", "baseline"]):
                        command = [str(binaries[side]), operation, "decode", profile, str(size),
                                   "random", "4096", str(rounds), "warm"]
                        output = subprocess.check_output(command, text=True, timeout=60)
                        parsed = list(csv.DictReader(io.StringIO(output)))
                        if len(parsed) != 1:
                            raise ValueError("expected one measurement row")
                        row = parse_sample(parsed[0], rounds, size, encoded)
                        if row["decode_capability"] != "rvv" or row["allocations"] != 0:
                            raise ValueError("native RVV capability or zero-allocation contract missing")
                        rows.append(dict(row, case=case, sample=sample, side=side, features="std,simd"))
                print(f"measured {operation}/{profile}/{size}", file=sys.stderr, flush=True)
    if any(hashlib.sha256(path.read_bytes()).hexdigest() != hashes[side] for side, path in binaries.items()):
        raise ValueError("benchmark binary changed during measurement")
    print(json.dumps(dict(scope="trusted native development comparison, not release admission",
                          host=platform.platform(), cpuinfo=Path("/proc/cpuinfo").read_text(),
                          binary_sha256=hashes, rows=rows, summary=summarize(rows)), indent=2))


if __name__ == "__main__":
    main()
