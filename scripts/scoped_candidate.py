"""Verify independently digest-approved, exact-candidate metadata and CI records."""

import re
import tomllib

from scoped_evidence import read_regular, strict_json


PREFIX = "target/release-evidence/"
REVIEW = "CANDIDATE-REVIEW.json"
CI = "candidate/ci.json"
RUST = "candidate/rust-packages.json"
NPM = "candidate/wasm-npm.json"
SBOMS = ("base64-ng.spdx.json", "base64-ng.cyclonedx.json")
REPRO_PAIRS = (
    ("release-a.rlib", "release-b.rlib"),
    ("package-files-a.txt", "package-files-b.txt"),
    ("base64-ng-a.crate", "base64-ng-b.crate"),
)


def exact_key(lines, key, value):
    if [line[len(key) + 1:] for line in lines if line.startswith(key + "=")] != [value]:
        raise ValueError("missing or duplicate metadata key: " + key)


def require_file(inventory, path):
    if path not in inventory["files"] or inventory["files"][path]["bytes"] == 0:
        raise ValueError("missing or empty candidate artifact: " + str(path))
    return inventory["files"][path]


def manifest(evidence, name, candidate, inventory, artifacts):
    require_file(inventory, name)
    lines = read_regular(evidence / name).decode().splitlines()
    if lines.count("source:") != 1:
        raise ValueError("candidate manifest source section mismatch")
    exact_key(lines, "commit", candidate["commit"])
    exact_key(lines, "tree_state", "clean")
    for path in artifacts:
        pin = require_file(inventory, path)
        expected = pin["sha256"] + "  " + PREFIX + path
        matches = [line for line in lines if line.endswith("  " + PREFIX + path)]
        if matches != [expected]:
            raise ValueError("candidate manifest artifact digest mismatch: " + path)
    return lines


def report(evidence, name, candidate, inventory):
    require_file(inventory, name)
    value = strict_json(read_regular(evidence / name))
    if value["schema"] != 1 or value["source"] != candidate or value["status"] != "passed":
        raise ValueError("candidate metadata report source/status mismatch: " + name)
    return value


def verify_ci(evidence, candidate, inventory, reviews):
    require_file(inventory, CI)
    value = strict_json(read_regular(evidence / CI))
    baseline = strict_json(read_regular(reviews / "github-ci.json"))
    expected = {job["name"] for job in baseline["jobs"]}
    jobs = value["jobs"]
    if (value["headSha"] != candidate["commit"] or value["event"] != "workflow_dispatch"
            or value["conclusion"] != "success" or value["workflowName"] != "Rust CI"
            or not re.fullmatch(r"https://github\.com/valkyoth/base64-ng/actions/runs/[1-9][0-9]*",
                                value["url"])
            or len(jobs) != len(expected) or {job["name"] for job in jobs} != expected
            or any(job["status"] != "completed" or job["conclusion"] != "success" for job in jobs)):
        raise ValueError("exact-candidate full-release CI did not pass the complete job inventory")


def verify_packages(root, evidence, candidate, inventory):
    plan = tomllib.loads(read_regular(root / "release-crates.toml").decode())
    rust = report(evidence, RUST, candidate, inventory)
    if plan["release"] != {"version": "2.1.0", "policy": "synced-family"}:
        raise ValueError("scoped activation requires the reviewed synchronized 2.1.0 family")
    if set(rust["packages"]) != set(plan["crates"]):
        raise ValueError("candidate Rust package family is incomplete")
    for name, item in rust["packages"].items():
        expected = {"version": "2.1.0", "archive": f"candidate/rust/{name}-2.1.0.crate",
                    "inventory": f"candidate/rust/{name}-files.txt"}
        if item != expected or plan["crates"][name]["version"] != "2.1.0":
            raise ValueError("candidate Rust package identity mismatch")
        for key in ("archive", "inventory"):
            require_file(inventory, item[key])
    npm = report(evidence, NPM, candidate, inventory)
    if npm["package"] != plan["npm"]["name"] or npm["version"] != "2.1.0":
        raise ValueError("candidate npm package identity mismatch")
    for name in ("package.tgz", "files.txt", "PROVENANCE.json"):
        require_file(inventory, "candidate/npm/" + name)
    provenance = strict_json(read_regular(evidence / "candidate/npm/PROVENANCE.json"))
    if (provenance["schema"] != "base64-ng-wasm-provenance-v1"
            or provenance["sourceCommit"] != candidate["commit"]
            or provenance["package"] != npm["package"] or provenance["version"] != "2.1.0"):
        raise ValueError("candidate WASM provenance mismatch")
    expected_artifacts = {"base64-ng-scalar.wasm", "base64-ng-simd128.wasm"}
    if set(provenance["artifacts"]) != expected_artifacts:
        raise ValueError("candidate WASM artifact inventory mismatch")
    from scoped_finalizer import record
    for name in expected_artifacts:
        pin = require_file(inventory, "candidate/npm/" + name)
        original = record(read_regular(root / "packages/base64-ng-wasm-loader/artifacts" / name))
        if provenance["artifacts"][name] != pin["sha256"] or pin != original:
            raise ValueError("candidate WASM differs from the reviewed kernel artifact")


def verify_metadata(root, evidence, candidate, inventory, reviews):
    verify_ci(evidence, candidate, inventory, reviews)
    manifest(evidence, "sbom-MANIFEST.txt", candidate, inventory, SBOMS)
    spdx, cyclone = [strict_json(read_regular(evidence / path)) for path in SBOMS]
    if spdx["spdxVersion"] != "SPDX-2.3" or cyclone["bomFormat"] != "CycloneDX" \
            or cyclone["specVersion"] != "1.4":
        raise ValueError("candidate SBOM formats mismatch")
    artifacts = ["reproducible/" + name for pair in REPRO_PAIRS for name in pair]
    lines = manifest(evidence, "reproducible/MANIFEST.txt", candidate, inventory, artifacts)
    exact_key(lines, "comparisons", "release-rlib,package-file-list,crate-archive")
    for left, right in REPRO_PAIRS:
        if inventory["files"]["reproducible/" + left] != inventory["files"]["reproducible/" + right]:
            raise ValueError("candidate reproducibility pair mismatch")
    verify_packages(root, evidence, candidate, inventory)
