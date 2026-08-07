#!/usr/bin/env python3
"""Fail-closed mutation tests for distributed fuzz evidence."""

from __future__ import annotations

import hashlib
import importlib.util
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "fuzz_shard_evidence", ROOT / "scripts" / "fuzz_shard_evidence.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

RVV_SPEC = importlib.util.spec_from_file_location(
    "test_rvv_admission_bundle", ROOT / "scripts" / "test-rvv-admission-bundle.py"
)
assert RVV_SPEC is not None and RVV_SPEC.loader is not None
RVV_FIXTURE = importlib.util.module_from_spec(RVV_SPEC)
sys.modules[RVV_SPEC.name] = RVV_FIXTURE
RVV_SPEC.loader.exec_module(RVV_FIXTURE)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def architecture(target: str) -> tuple[str, str, str, str]:
    if target in {"x86_encode", "x86_decode"}:
        return (
            "x86_64-avx512vbmi",
            "x86_64",
            "little",
            "avx512f,avx512bw,avx512vl,avx512vbmi",
        )
    if target == "neon":
        return ("aarch64-neon", "aarch64", "little", "neon")
    return ("portable", "x86_64", "little", "not-required")


def write_bundle(root: Path, target: str, source: object) -> Path:
    bundle = root / target
    bundle.mkdir(parents=True)
    log = bundle / "campaign.log"
    environment = bundle / "environment.txt"
    archive = bundle / "corpus.tar.gz"
    corpus = root / f".{target}-corpus"
    corpus.mkdir()
    (corpus / "seed").write_bytes(target.encode())
    with tarfile.open(archive, "w:gz") as output:
        output.add(corpus / "seed", arcname="seed")
    shutil.rmtree(corpus)
    log.write_text(
        "stat::number_of_executed_units: 100\n"
        "stat::average_exec_per_sec: 10\n"
    )
    environment.write_text(f"fixture target={target}\n")
    arch_class, host_arch, endian, features = architecture(target)
    values = {
        "schema": MODULE.SCHEMA,
        "target": target,
        "status": "ok",
        "source_commit": source.commit,
        "source_tree": source.tree,
        "tree_state": "clean",
        "cargo_lock_sha256": source.cargo_lock,
        "fuzz_lock_sha256": source.fuzz_lock,
        "fuzz_manifest_sha256": source.fuzz_manifest,
        "harness_sha256": sha256(ROOT / "fuzz" / "fuzz_targets" / f"{target}.rs"),
        "duration_seconds": "3600",
        "started_epoch": "100",
        "finished_epoch": "3700",
        "elapsed_seconds": "3600",
        "architecture_class": arch_class,
        "host_arch": host_arch,
        "host_endian": endian,
        "rustc_host": f"{host_arch}-unknown-linux-gnu",
        "cpu_features": features,
        "machine_label": "fixture",
        "corpus_count": "1",
        "artifact_count": "0",
        "log_sha256": sha256(log),
        "environment_sha256": sha256(environment),
        "corpus_archive_sha256": sha256(archive),
    }
    (bundle / "MANIFEST.txt").write_text(
        "".join(f"{key}={value}\n" for key, value in values.items())
    )
    return bundle


def must_reject(bundle: Path, source: object, description: str) -> None:
    try:
        MODULE.validate_bundle(bundle, source)
    except MODULE.EvidenceError:
        return
    raise AssertionError(f"accepted {description}")


def replace_key(manifest: Path, key: str, value: str) -> None:
    lines = manifest.read_text().splitlines()
    manifest.write_text(
        "\n".join(f"{key}={value}" if line.startswith(f"{key}=") else line for line in lines)
        + "\n"
    )


def clone_bundle(source: Path, root: Path, name: str) -> Path:
    destination = root / name / source.name
    destination.parent.mkdir(parents=True)
    shutil.copytree(source, destination)
    return destination


def main() -> None:
    temporary = Path(tempfile.mkdtemp(prefix="base64-ng-fuzz-shard-test-"))
    try:
        source = MODULE.current_source(require_clean=False)
        with (ROOT / "fuzz" / "Cargo.toml").open("rb") as manifest:
            cargo_targets = [entry["name"] for entry in tomllib.load(manifest)["bin"]]
        assert cargo_targets == MODULE.targets()
        single = temporary / "single"
        original = write_bundle(single, "decode", source)
        MODULE.validate_bundle(original, source)

        mutations = temporary / "mutations"
        changed = clone_bundle(original, mutations, "log")
        (changed / "campaign.log").write_text("tampered\n")
        must_reject(changed, source, "tampered log")

        changed = clone_bundle(original, mutations, "short")
        replace_key(changed / "MANIFEST.txt", "duration_seconds", "3599")
        must_reject(changed, source, "short campaign")

        changed = clone_bundle(original, mutations, "commit")
        replace_key(changed / "MANIFEST.txt", "source_commit", "0" * 40)
        must_reject(changed, source, "wrong commit")

        changed = clone_bundle(original, mutations, "artifact")
        replace_key(changed / "MANIFEST.txt", "artifact_count", "1")
        must_reject(changed, source, "crash artifact")

        changed = clone_bundle(original, mutations, "duplicate")
        with (changed / "MANIFEST.txt").open("a") as manifest:
            manifest.write("status=ok\n")
        must_reject(changed, source, "duplicate manifest key")

        changed = clone_bundle(original, mutations, "extra")
        (changed / "untracked.txt").write_text("not covered by the bundle contract\n")
        must_reject(changed, source, "extra untracked bundle file")

        x86 = write_bundle(temporary / "x86", "x86_encode", source)
        replace_key(x86 / "MANIFEST.txt", "cpu_features", "avx2")
        must_reject(x86, source, "x86 shard without AVX-512 VBMI")

        neon = write_bundle(temporary / "neon", "neon", source)
        replace_key(neon / "MANIFEST.txt", "host_arch", "x86_64")
        must_reject(neon, source, "NEON shard from the wrong architecture")

        collection = temporary / "collection"
        for target in MODULE.targets():
            write_bundle(collection, target, source)
        evidence = temporary / "evidence"
        destination = evidence / "fuzz"
        MODULE.aggregate(collection, destination=destination, source=source)
        aggregate_manifest = (destination / "MANIFEST.txt").read_text()
        aggregate_lines = aggregate_manifest.splitlines()
        for target in MODULE.targets():
            assert aggregate_lines.count(f"{target}=ok corpus=1 artifacts=0") == 1

        fixtures = {
            "miri/MANIFEST.txt": (
                "no_default_features=0\nall_features=0\nbase64_ng_bytes=0\n"
                "base64_ng_tokio_readers=0\nbase64_ng_tokio_writers=0\n"
            ),
            "2.0-memory-sanitizers/MANIFEST.txt": (
                "address_status=0\nleak_status=0\nthread_status=0\n"
                "target=x86_64-unknown-linux-gnu\n"
            ),
            "dudect/MANIFEST.txt": (
                "samples=20000\niterations=64\nwarmup=1000\nthreshold=10\nstatus=0\n"
            ),
            "backend/MANIFEST.txt": (
                "runtime_backend_report=0\nsimd_prototype_equivalence=0\n"
            ),
            "kani/normal/status.txt": "PASS\n",
            "kani/advanced/status.txt": "PASS\n",
            "commit-53/MANIFEST.txt": (
                "neon_automatic_dispatch=retained-native-performance\n"
                "rvv=exact-linux-spacemit-x60-native-admission\n"
            ),
        }
        for relative, content in fixtures.items():
            path = evidence / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        RVV_FIXTURE.write_bundle(evidence / "riscv-native-admission")
        subprocess.run(
            [str(ROOT / "scripts" / "validate-release-evidence-outcomes.sh"), str(evidence)],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )

        missing = temporary / "missing"
        shutil.copytree(collection, missing)
        shutil.rmtree(missing / "v2_assurance")
        try:
            MODULE.aggregate(missing, destination=temporary / "rejected", source=source)
        except MODULE.EvidenceError:
            pass
        else:
            raise AssertionError("accepted incomplete collection")

        unknown = temporary / "unknown"
        shutil.copytree(collection, unknown)
        (unknown / "extra").mkdir()
        try:
            MODULE.aggregate(unknown, destination=temporary / "rejected-extra", source=source)
        except MODULE.EvidenceError:
            pass
        else:
            raise AssertionError("accepted unknown or duplicate-equivalent target")
    finally:
        shutil.rmtree(temporary)
    print("fuzz shard evidence tests: ok")


if __name__ == "__main__":
    main()
