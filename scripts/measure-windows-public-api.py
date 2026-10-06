#!/usr/bin/env python3
"""Trusted native Windows public-call comparison; not final release admission."""
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys

from public_api_baseline import parse_sample, summarize


def capture(binary, samples=15, sizes=(32, 768, 65536, 1048576)):
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    names = subprocess.check_output([str(binary), "list"], text=True, timeout=30).splitlines()
    required = {"canonical", "historical", "canonical-reference", "historical-reference", "scalar"}
    if not required <= set(names):
        raise ValueError("missing public validation-policy benchmark operations")
    exact = [name for name in ("ssse3-sse4.1", "avx2", "avx512-vbmi") if name in names]
    rows = []
    for direction in ("encode", "decode"):
        pairs = [("scalar", name) for name in ("canonical", "historical", *exact)]
        if direction == "decode":
            pairs += [(name + "-reference", name) for name in ("canonical", "historical")]
        for baseline, candidate in pairs:
            for profile in ("sp", "su", "up", "uu"):
                for size in sizes:
                    rounds = 1000 if size <= 768 else 20
                    encoded = (size + 2) // 3 * 4 if profile.endswith("p") else (size * 8 + 5) // 6
                    case = [baseline + "->" + candidate, direction, profile, size, "random", 4096, "warm"]
                    for sample in range(samples):
                        order = [("baseline", baseline), ("candidate", candidate)]
                        if sample % 2:
                            order.reverse()
                        for side, operation in order:
                            command = [str(binary), operation, direction, profile, str(size),
                                       "random", "4096", str(rounds), "warm"]
                            output = subprocess.check_output(command, text=True, timeout=120)
                            parsed = list(csv.DictReader(io.StringIO(output)))
                            if len(parsed) != 1:
                                raise ValueError("expected exactly one benchmark row")
                            row = parse_sample(parsed[0], rounds, size, encoded)
                            if row["allocations"] != 0:
                                raise ValueError("unexpected timed allocation")
                            rows.append(dict(row, case=case, sample=sample, side=side,
                                             features="std,simd,validation-policy"))
    if hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
        raise ValueError("benchmark executable changed during capture")
    return dict(scope=__doc__, binary_sha256=digest, exact_backends=exact,
                unavailable_backends=sorted({"ssse3-sse4.1", "avx2", "avx512-vbmi"} - set(exact)),
                rows=rows, summary=summarize(rows))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("native Windows capture required")
    report = capture(args.binary.resolve())
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(report, stream, indent=2)


if __name__ == "__main__":
    main()
