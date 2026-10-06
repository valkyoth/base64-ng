#!/usr/bin/env python3
"""Paired policy capture of explicitly trusted, prebuilt native executables.

This is not the Linux untrusted-candidate sandbox. Build both reviewed sources
with the same compiler and harness before use. Binary hashes bind the capture;
source/build provenance must accompany it separately.
"""
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import platform

from public_api_baseline import parse_sample, summarize
from public_api_policy import iterations, operation, policy_cases
from public_api_sandbox import bounded


def capture(baseline, candidate, *, controls=False, samples=15):
    if not 15 <= samples <= 100:
        raise ValueError("policy capture requires 15..100 pairs")
    binaries = {"baseline": baseline.resolve(), "candidate": candidate.resolve()}
    hashes = {side: hashlib.sha256(path.read_bytes()).hexdigest() for side, path in binaries.items()}
    if controls and hashes["baseline"] != hashes["candidate"]:
        raise ValueError("same-source controls require the same executable")
    available = {side: bounded([str(path), "list"]).decode().splitlines()
                 for side, path in binaries.items()}
    if available["baseline"] != available["candidate"]:
        raise ValueError("operation availability changed")
    diagnostics = {side: bounded([str(path), "diagnostics"], limit=8 * 1024 * 1024)
                   for side, path in binaries.items()}
    if diagnostics["baseline"] != diagnostics["candidate"]:
        raise ValueError("diagnostic or destination-mutation regression")
    rows = []
    for case in policy_cases(available["baseline"], controls):
        size = int(case[3])
        encoded = (size + 2) // 3 * 4 if case[2].endswith("p") else (size * 8 + 5) // 6
        rounds = iterations(case)
        for sample in range(samples):
            for side in (("baseline", "candidate") if sample % 2 == 0 else ("candidate", "baseline")):
                raw = bounded([str(binaries[side]), operation(case, side), *case[1:-1],
                               str(rounds), case[-1]], timeout=120).decode()
                parsed = list(csv.DictReader(io.StringIO(raw)))
                if len(parsed) != 1:
                    raise ValueError("expected exactly one measurement row")
                row = parse_sample(parsed[0], rounds, size, encoded)
                rows.append(dict(row, case=case, sample=sample, side=side, features="operator-built"))
    if any(hashlib.sha256(path.read_bytes()).hexdigest() != hashes[side] for side, path in binaries.items()):
        raise ValueError("benchmark executable changed during capture")
    return dict(schema=1, scope=__doc__, comparison="controls" if controls else "revisions",
                os=platform.system(), os_release=platform.release(), machine=platform.machine(),
                binary_sha256=hashes, operations=available["baseline"],
                diagnostics_sha256=hashlib.sha256(diagnostics["baseline"]).hexdigest(),
                rows=rows, summary=summarize(rows))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--trusted-binaries", action="store_true", required=True)
    parser.add_argument("--controls", action="store_true")
    args = parser.parse_args()
    # Reserve the output before expensive execution; never replace an old capture.
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(capture(args.baseline, args.candidate, controls=args.controls), stream, indent=2)
        stream.write("\n")


if __name__ == "__main__":
    main()
