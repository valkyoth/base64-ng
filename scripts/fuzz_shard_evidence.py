#!/usr/bin/env python3
"""Validate and aggregate exact-source distributed fuzz evidence."""

from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TARGET_FILE = ROOT / "scripts" / "fuzz-release-targets.txt"
SCHEMA = "base64-ng-fuzz-shard-v1"
MINIMUM_SECONDS = 3600
REQUIRED_FILES = ("MANIFEST.txt", "campaign.log", "environment.txt", "corpus.tar.gz")
REQUIRED_KEYS = {
    "schema",
    "target",
    "status",
    "source_commit",
    "source_tree",
    "tree_state",
    "cargo_lock_sha256",
    "fuzz_lock_sha256",
    "fuzz_manifest_sha256",
    "harness_sha256",
    "duration_seconds",
    "started_epoch",
    "finished_epoch",
    "elapsed_seconds",
    "architecture_class",
    "host_arch",
    "host_endian",
    "rustc_host",
    "cpu_features",
    "machine_label",
    "corpus_count",
    "artifact_count",
    "log_sha256",
    "environment_sha256",
    "corpus_archive_sha256",
}


class EvidenceError(RuntimeError):
    pass


@dataclass(frozen=True)
class Source:
    commit: str
    tree: str
    cargo_lock: str
    fuzz_lock: str
    fuzz_manifest: str


@dataclass(frozen=True)
class Bundle:
    path: Path
    values: dict[str, str]

    @property
    def target(self) -> str:
        return self.values["target"]


def fail(message: str) -> None:
    raise EvidenceError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def targets() -> list[str]:
    values = [line.strip() for line in TARGET_FILE.read_text().splitlines() if line.strip()]
    if len(values) != len(set(values)) or len(values) != 18:
        fail("target inventory must contain 18 unique names")
    return values


def run_git(*arguments: str) -> str:
    result = subprocess.run(
        ["git", *arguments], cwd=ROOT, check=True, text=True, capture_output=True
    )
    return result.stdout.strip()


def current_source(require_clean: bool = True) -> Source:
    if require_clean and run_git("status", "--porcelain", "--untracked-files=all"):
        fail("aggregation requires a clean worktree")
    return Source(
        commit=run_git("rev-parse", "--verify", "HEAD"),
        tree=run_git("rev-parse", "HEAD^{tree}"),
        cargo_lock=sha256(ROOT / "Cargo.lock"),
        fuzz_lock=sha256(ROOT / "fuzz" / "Cargo.lock"),
        fuzz_manifest=sha256(ROOT / "fuzz" / "Cargo.toml"),
    )


def parse_manifest(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for number, raw in enumerate(path.read_text().splitlines(), start=1):
        if not raw or raw.startswith("#"):
            continue
        if "=" not in raw:
            fail(f"{path}: line {number} is not key=value")
        key, value = raw.split("=", 1)
        if not key or key in values:
            fail(f"{path}: empty or duplicate key {key!r}")
        values[key] = value
    missing = REQUIRED_KEYS - values.keys()
    unknown = values.keys() - REQUIRED_KEYS
    if missing or unknown:
        fail(f"{path}: missing={sorted(missing)} unknown={sorted(unknown)}")
    return values


def integer(values: dict[str, str], key: str) -> int:
    try:
        value = int(values[key], 10)
    except ValueError as error:
        raise EvidenceError(f"{key} must be an integer") from error
    if value < 0 or str(value) != values[key]:
        fail(f"{key} must be a canonical non-negative integer")
    return value


def expected_architecture(target: str) -> str:
    if target in {"x86_encode", "x86_decode"}:
        return "x86_64-avx512vbmi"
    if target == "neon":
        return "aarch64-neon"
    return "portable"


def validate_archive(path: Path, expected_files: int) -> None:
    try:
        with tarfile.open(path, "r:gz") as archive:
            members = archive.getmembers()
    except (tarfile.TarError, OSError) as error:
        raise EvidenceError(f"invalid corpus archive {path}: {error}") from error
    files = 0
    for member in members:
        member_path = Path(member.name)
        if member_path.is_absolute() or ".." in member_path.parts:
            fail(f"unsafe corpus archive path: {member.name}")
        if member.issym() or member.islnk() or member.isdev():
            fail(f"unsupported corpus archive member: {member.name}")
        files += int(member.isfile())
    if files != expected_files:
        fail(f"corpus archive has {files} files, manifest reports {expected_files}")


def validate_bundle(path: Path, source: Source | None = None) -> Bundle:
    if not path.is_dir():
        fail(f"missing bundle directory: {path}")
    entries = sorted(entry.name for entry in path.iterdir())
    if entries != sorted(REQUIRED_FILES):
        fail(f"{path}: bundle must contain exactly {sorted(REQUIRED_FILES)}")
    for name in REQUIRED_FILES:
        file = path / name
        if not file.is_file() or file.stat().st_size == 0:
            fail(f"missing or empty bundle file: {file}")
    if (path / "campaign.log").stat().st_size > 64 * 1024 * 1024:
        fail(f"{path}: campaign log exceeds 64 MiB")
    if (path / "environment.txt").stat().st_size > 4 * 1024 * 1024:
        fail(f"{path}: environment record exceeds 4 MiB")
    if (path / "corpus.tar.gz").stat().st_size > 1024 * 1024 * 1024:
        fail(f"{path}: corpus archive exceeds 1 GiB")
    values = parse_manifest(path / "MANIFEST.txt")
    target = values["target"]
    if target not in targets() or path.name != target:
        fail(f"bundle target/path mismatch: {target} at {path}")
    if values["schema"] != SCHEMA or values["status"] != "ok":
        fail(f"{target}: invalid schema or non-success status")
    if values["tree_state"] != "clean":
        fail(f"{target}: release evidence requires a clean source tree")
    duration = integer(values, "duration_seconds")
    started = integer(values, "started_epoch")
    finished = integer(values, "finished_epoch")
    elapsed = integer(values, "elapsed_seconds")
    corpus_count = integer(values, "corpus_count")
    if corpus_count > 100_000:
        fail(f"{target}: corpus count exceeds the evidence review ceiling")
    if duration < MINIMUM_SECONDS or elapsed < duration or finished - started != elapsed:
        fail(f"{target}: shortened or inconsistent campaign timing")
    if integer(values, "artifact_count") != 0:
        fail(f"{target}: crash artifacts are present")
    expected_class = expected_architecture(target)
    if values["architecture_class"] != expected_class:
        fail(f"{target}: expected architecture class {expected_class}")
    if expected_class == "x86_64-avx512vbmi":
        required = {"avx512f", "avx512bw", "avx512vl", "avx512vbmi"}
        if values["host_arch"] not in {"x86_64", "amd64"} or not required.issubset(
            set(values["cpu_features"].split(","))
        ):
            fail(f"{target}: complete AVX-512 VBMI feature evidence is missing")
    if expected_class == "aarch64-neon" and (
        values["host_arch"] not in {"aarch64", "arm64"}
        or values["host_endian"] != "little"
        or "neon" not in values["cpu_features"].split(",")
    ):
        fail(f"{target}: native little-endian AArch64 NEON evidence is missing")
    if not re.fullmatch(r"[A-Za-z0-9._-]+", values["machine_label"]):
        fail(f"{target}: invalid machine label")
    hash_fields = {
        "log_sha256": path / "campaign.log",
        "environment_sha256": path / "environment.txt",
        "corpus_archive_sha256": path / "corpus.tar.gz",
    }
    for key, file in hash_fields.items():
        if values[key] != sha256(file):
            fail(f"{target}: {key} does not match {file.name}")
    log = (path / "campaign.log").read_text(errors="replace")
    for marker in ("stat::number_of_executed_units:", "stat::average_exec_per_sec:"):
        if marker not in log:
            fail(f"{target}: final libFuzzer statistic is missing: {marker}")
    validate_archive(path / "corpus.tar.gz", corpus_count)
    if source is not None:
        expected = {
            "source_commit": source.commit,
            "source_tree": source.tree,
            "cargo_lock_sha256": source.cargo_lock,
            "fuzz_lock_sha256": source.fuzz_lock,
            "fuzz_manifest_sha256": source.fuzz_manifest,
            "harness_sha256": sha256(ROOT / "fuzz" / "fuzz_targets" / f"{target}.rs"),
        }
        for key, value in expected.items():
            if values[key] != value:
                fail(f"{target}: {key} does not match the current exact source")
    return Bundle(path=path, values=values)


def bundle_files(bundle: Bundle) -> list[Path]:
    return [bundle.path / name for name in REQUIRED_FILES]


def aggregate(
    collection: Path, destination: Path | None = None, source: Source | None = None
) -> None:
    source = source or current_source()
    expected = targets()
    directories = sorted(path.name for path in collection.iterdir() if path.is_dir())
    if directories != sorted(expected):
        fail("collection must contain exactly one directory for every release target")
    bundles = [validate_bundle(collection / target, source) for target in expected]
    destination = destination or ROOT / "target" / "release-evidence" / "fuzz"
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=".fuzz-aggregate-", dir=destination.parent))
    try:
        shards = temporary / "shards"
        shards.mkdir()
        for bundle in bundles:
            shutil.copy2(bundle.path / "campaign.log", temporary / f"{bundle.target}.txt")
            shutil.copytree(bundle.path, shards / bundle.target)
        minimum = min(integer(bundle.values, "duration_seconds") for bundle in bundles)
        with (temporary / "MANIFEST.txt").open("w", encoding="utf-8") as manifest:
            manifest.write("base64-ng fuzz campaign evidence\n\nsource:\n")
            manifest.write(f"commit={source.commit}\ntree_state=clean\n")
            manifest.write(f"{source.cargo_lock}  Cargo.lock\n\nparameters:\n")
            manifest.write("mode=release-duration\n")
            manifest.write(f"campaign_argument=-max_total_time={minimum}\n")
            manifest.write("distribution=verified-per-target-shards\n")
            manifest.write("panic_oracle=crate-originated panic is a campaign failure\n")
            manifest.write("artifact_oracle=zero artifacts required\n\ntargets:\n")
            for bundle in bundles:
                count = integer(bundle.values, "corpus_count")
                manifest.write(f"{bundle.target}=ok corpus={count} artifacts=0\n")
            manifest.write("\ncampaign-output-hashes:\n")
            for bundle in bundles:
                output = temporary / f"{bundle.target}.txt"
                manifest.write(f"{sha256(output)}  {bundle.target}.txt\n")
            manifest.write("\nshard-bundle-hashes:\n")
            for bundle in bundles:
                for file in bundle_files(Bundle(shards / bundle.target, bundle.values)):
                    relative = file.relative_to(temporary)
                    manifest.write(f"{sha256(file)}  {relative.as_posix()}\n")
            manifest.write("\nminimization: no crashing artifact remained\n")
        if destination.exists():
            shutil.rmtree(destination)
        temporary.rename(destination)
    except BaseException:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    print(f"fuzz shard evidence: aggregated {len(bundles)} targets into {destination}")


def progress(collection: Path) -> None:
    source = current_source()
    if not collection.is_dir():
        fail(f"missing collection directory: {collection}")
    unknown = sorted(
        path.name for path in collection.iterdir() if path.is_dir() and path.name not in targets()
    )
    if unknown:
        fail(f"unknown bundle directories: {unknown}")
    complete: list[str] = []
    missing: list[str] = []
    for target in targets():
        path = collection / target
        if path.exists():
            validate_bundle(path, source)
            complete.append(target)
        else:
            missing.append(target)
    print(f"fuzz shard evidence: complete={len(complete)} missing={len(missing)}")
    if complete:
        print("complete: " + " ".join(complete))
    if missing:
        print("missing: " + " ".join(missing))


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate_parser = subparsers.add_parser("validate")
    validate_parser.add_argument("bundle", type=Path)
    progress_parser = subparsers.add_parser("progress")
    progress_parser.add_argument("collection", type=Path)
    aggregate_parser = subparsers.add_parser("aggregate")
    aggregate_parser.add_argument("collection", type=Path)
    arguments = parser.parse_args()
    try:
        if arguments.command == "validate":
            validate_bundle(arguments.bundle, current_source())
            print(f"fuzz shard evidence: valid bundle {arguments.bundle}")
        elif arguments.command == "progress":
            progress(arguments.collection)
        else:
            aggregate(arguments.collection)
    except (EvidenceError, OSError, subprocess.CalledProcessError) as error:
        print(f"fuzz shard evidence: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
