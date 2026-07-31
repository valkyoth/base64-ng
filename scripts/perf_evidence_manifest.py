"""Validate performance-evidence manifest metadata and artifact digests."""

from __future__ import annotations

import hashlib
import re
from pathlib import Path

from perf_evidence_schema import MANIFEST_ARTIFACTS, MANIFEST_STATIC_METADATA

HEADER = "base64-ng performance evidence schema 1"
CHECKSUM = re.compile(r"([0-9a-f]{64})  (.+)\Z")


def validate_manifest(directory: Path, environment: dict[str, object]) -> None:
    manifest = directory / "MANIFEST.txt"
    try:
        lines = manifest.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise ValueError(f"{manifest}: cannot read manifest: {error}") from error
    if not lines or lines[0] != HEADER:
        raise ValueError(f"{manifest}: unsupported manifest schema")
    if lines.count("artifacts:") != 1:
        raise ValueError(f"{manifest}: expected exactly one artifacts marker")

    marker = lines.index("artifacts:")
    metadata: dict[str, str] = {}
    for line in lines[1:marker]:
        if "=" not in line:
            raise ValueError(f"{manifest}: malformed metadata line {line!r}")
        key, value = line.split("=", 1)
        if not key or key in metadata:
            raise ValueError(f"{manifest}: duplicate or empty metadata key {key!r}")
        metadata[key] = value

    source = environment["source"]
    measurement = environment["measurement"]
    if not isinstance(source, dict) or not isinstance(measurement, dict):
        raise ValueError(f"{manifest}: environment contract disappeared")
    expected_metadata = {
        "source_commit": str(source["commit"]),
        "source_status": str(source["status"]),
        "campaign_id": str(measurement["campaign_id"]),
        "sample_count": str(measurement["sample_count"]),
        "target_bytes_per_sample": str(measurement["target_bytes_per_sample"]),
        **MANIFEST_STATIC_METADATA,
    }
    if metadata != expected_metadata:
        missing = sorted(expected_metadata.keys() - metadata.keys())
        extra = sorted(metadata.keys() - expected_metadata.keys())
        changed = sorted(
            key
            for key in expected_metadata.keys() & metadata.keys()
            if metadata[key] != expected_metadata[key]
        )
        raise ValueError(
            f"{manifest}: metadata mismatch: missing={missing}, "
            f"extra={extra}, changed={changed}"
        )

    observed: dict[str, str] = {}
    for line in lines[marker + 1 :]:
        match = CHECKSUM.fullmatch(line)
        if match is None:
            raise ValueError(f"{manifest}: malformed checksum line {line!r}")
        digest, recorded_path = match.groups()
        artifact = Path(recorded_path).name
        if artifact in observed:
            raise ValueError(f"{manifest}: duplicate artifact {artifact!r}")
        observed[artifact] = digest
    if set(observed) != MANIFEST_ARTIFACTS:
        missing = sorted(MANIFEST_ARTIFACTS - observed.keys())
        extra = sorted(observed.keys() - MANIFEST_ARTIFACTS)
        raise ValueError(
            f"{manifest}: artifact inventory mismatch: "
            f"missing={missing}, extra={extra}"
        )
    for artifact, expected_digest in observed.items():
        path = directory / artifact
        try:
            actual_digest = hashlib.sha256(path.read_bytes()).hexdigest()
        except OSError as error:
            raise ValueError(f"{manifest}: cannot hash {artifact}: {error}") from error
        if actual_digest != expected_digest:
            raise ValueError(f"{manifest}: checksum mismatch for {artifact}")
