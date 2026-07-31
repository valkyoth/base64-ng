#!/usr/bin/env python3
"""Mutation tests for the performance-evidence trust boundary."""

from __future__ import annotations

import csv
import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
from pathlib import Path
from types import ModuleType
from typing import Callable

ROOT = Path(__file__).resolve().parent.parent
VALIDATOR_PATH = ROOT / "scripts" / "validate_perf_evidence.py"
CAPTURE_PATH = ROOT / "scripts" / "capture_perf_environment.py"


def load_validator() -> ModuleType:
    spec = importlib.util.spec_from_file_location(
        "base64_ng_validate_perf_evidence", VALIDATOR_PATH
    )
    if spec is None or spec.loader is None:
        raise SystemExit("performance evidence tests: cannot load validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


VALIDATOR = load_validator()


def environment() -> dict[str, object]:
    return {
        "schema_version": 1,
        "source": {"commit": "0" * 40, "status": "clean"},
        "measurement": {
            "campaign_id": "mutation-test",
            "sample_count": 2,
            "target_bytes_per_sample": 1024,
        },
    }


def availability_rows() -> list[dict[str, str]]:
    return [
        {
            "schema_version": "1",
            "backend": backend,
            "available": "true" if backend in {"auto", "scalar"} else "false",
            "target_arch": "x86_64",
            "target_os": "linux",
        }
        for backend in sorted(VALIDATOR.BACKENDS)
    ]


def resource_rows(feature_set: str = "simd") -> list[dict[str, str]]:
    return [
        {
            "schema_version": "1",
            "category": category,
            "name": name,
            "feature_set": feature_set,
            "value": "1",
            "unit": unit,
            "method": method,
        }
        for category, name, unit, method in sorted(VALIDATOR.RESOURCE_CONTRACT)
    ]


def benchmark_rows() -> list[dict[str, str]]:
    rows = []
    engines = (
        ("base64-ng", "auto"),
        ("base64-ng", "scalar"),
        ("base64-0.23.0", "external"),
        ("base64ct-1.8.3", "external"),
    )
    for operation in sorted(VALIDATOR.OPERATIONS):
        for alphabet, padding in sorted(VALIDATOR.PROFILES):
            for input_len in sorted(VALIDATOR.EXPECTED_LENGTHS, key=int):
                raw_len = int(input_len)
                encoded_len = (
                    ((raw_len + 2) // 3) * 4
                    if padding == "padded"
                    else (raw_len * 4 + 2) // 3
                )
                iterations = max(1024 // raw_len, 1)
                elapsed_ns = 1_000_000_000
                throughput = (
                    raw_len
                    * iterations
                    * 1_000_000_000.0
                    / (1024.0 * 1024.0 * elapsed_ns)
                )
                for engine, backend in engines:
                    for sample_index in range(2):
                        rows.append(
                            {
                                "schema_version": "1",
                                "campaign_id": "mutation-test",
                                "run_id": "run-1",
                                "sample_index": str(sample_index),
                                "engine": engine,
                                "operation": operation,
                                "alphabet": alphabet,
                                "padding": padding,
                                "input_len": input_len,
                                "encoded_len": str(encoded_len),
                                "iterations": str(iterations),
                                "elapsed_ns": str(elapsed_ns),
                                "throughput_mib_s": f"{throughput:.6f}",
                                "backend": backend,
                                "active_encode_backend": "scalar",
                                "active_decode_backend": "scalar",
                                "target_arch": "x86_64",
                                "target_os": "linux",
                                "allocation_count": "0",
                            }
                        )
    return rows


def write_csv(path: Path, fields: list[str], rows: list[dict[str, str]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def write_fixture(
    directory: Path,
    env: dict[str, object],
    availability: list[dict[str, str]],
    resources: list[dict[str, str]],
    benchmarks: list[dict[str, str]],
) -> None:
    (directory / "environment.json").write_text(
        json.dumps(env) + "\n", encoding="utf-8"
    )
    write_csv(
        directory / "availability.csv", VALIDATOR.AVAILABILITY_FIELDS, availability
    )
    write_csv(directory / "resources.csv", VALIDATOR.RESOURCE_FIELDS, resources)
    write_csv(directory / "raw.csv", VALIDATOR.BENCHMARK_FIELDS, benchmarks)


def validate_fixture(directory: Path) -> None:
    env = VALIDATOR.read_environment(directory / "environment.json")
    availability = VALIDATOR.validate_auxiliary(
        directory / "availability.csv", directory / "resources.csv", "simd"
    )
    VALIDATOR.validate_benchmark(
        directory / "raw.csv",
        availability_rows=availability,
        environment=env,
        expected_run_id="run-1",
    )


def expect_failure(
    mutate: Callable[
        [
            dict[str, object],
            list[dict[str, str]],
            list[dict[str, str]],
            list[dict[str, str]],
        ],
        None,
    ],
    expected: str,
) -> None:
    env = environment()
    availability = availability_rows()
    resources = resource_rows()
    benchmarks = benchmark_rows()
    mutate(env, availability, resources, benchmarks)
    with tempfile.TemporaryDirectory(prefix="base64-ng-perf-mutation-") as temporary:
        directory = Path(temporary)
        write_fixture(directory, env, availability, resources, benchmarks)
        try:
            validate_fixture(directory)
        except SystemExit as error:
            if expected not in str(error):
                raise SystemExit(
                    f"performance evidence tests: expected {expected!r}, got {error!r}"
                ) from error
        else:
            raise SystemExit(
                f"performance evidence tests: mutation passed: {expected}"
            )


def test_capture_requires_clean_commit() -> None:
    with tempfile.TemporaryDirectory(prefix="base64-ng-perf-capture-") as temporary:
        directory = Path(temporary)
        subprocess.run(["git", "init", "-q"], cwd=directory, check=True)
        subprocess.run(
            ["git", "config", "user.email", "evidence@example.invalid"],
            cwd=directory,
            check=True,
        )
        subprocess.run(
            ["git", "config", "user.name", "Evidence Test"],
            cwd=directory,
            check=True,
        )
        (directory / "tracked").write_text("tracked\n", encoding="utf-8")
        subprocess.run(["git", "add", "tracked"], cwd=directory, check=True)
        subprocess.run(["git", "commit", "-q", "-m", "fixture"], cwd=directory, check=True)
        commit = subprocess.run(
            ["git", "rev-parse", "HEAD^{commit}"],
            cwd=directory,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        capture_env = os.environ.copy()
        capture_env["BASE64_NG_PERF_SOURCE_COMMIT"] = commit

        output = directory.parent / f"{directory.name}-environment.json"
        subprocess.run(
            [str(CAPTURE_PATH), str(output)],
            cwd=directory,
            env=capture_env,
            check=True,
        )
        captured = json.loads(output.read_text(encoding="utf-8"))
        output.unlink()
        if captured["source"]["status"] != "clean":
            raise SystemExit("performance evidence tests: clean capture was not clean")
        if len(captured["source"]["commit"]) != 40:
            raise SystemExit("performance evidence tests: capture did not use full commit")

        wrong_env = capture_env.copy()
        wrong_env["BASE64_NG_PERF_SOURCE_COMMIT"] = "1" * 40
        result = subprocess.run(
            [str(CAPTURE_PATH), str(output)],
            cwd=directory,
            env=wrong_env,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode == 0 or "changed before capture" not in result.stderr:
            raise SystemExit("performance evidence tests: changed source was accepted")

        (directory / "untracked").write_text("dirty\n", encoding="utf-8")
        result = subprocess.run(
            [str(CAPTURE_PATH), str(output)],
            cwd=directory,
            env=capture_env,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode == 0 or "dirty tree" not in result.stderr:
            raise SystemExit("performance evidence tests: dirty capture was accepted")


def write_bundle(directory: Path) -> None:
    env = environment()
    first = benchmark_rows()
    second = [{**row, "run_id": "run-2"} for row in first]
    (directory / "environment.json").write_text(
        json.dumps(env) + "\n", encoding="utf-8"
    )
    write_csv(
        directory / "availability.csv",
        VALIDATOR.AVAILABILITY_FIELDS,
        availability_rows(),
    )
    write_csv(
        directory / "resources-default.csv",
        VALIDATOR.RESOURCE_FIELDS,
        resource_rows("simd"),
    )
    write_csv(
        directory / "resources-no-simd.csv",
        VALIDATOR.RESOURCE_FIELDS,
        resource_rows("no-simd"),
    )
    write_csv(directory / "raw-run-1.csv", VALIDATOR.BENCHMARK_FIELDS, first)
    write_csv(directory / "raw-run-2.csv", VALIDATOR.BENCHMARK_FIELDS, second)
    summary = [
        dict(zip(VALIDATOR.SUMMARY_FIELDS, row, strict=True))
        for row in VALIDATOR.summary_rows(first, second)
    ]
    admitted = [
        dict(zip(VALIDATOR.ADMISSION_FIELDS, row, strict=True))
        for row in VALIDATOR.admission_rows(first, second, 0.95)
    ]
    write_csv(directory / "summary.csv", VALIDATOR.SUMMARY_FIELDS, summary)
    write_csv(directory / "admission.csv", VALIDATOR.ADMISSION_FIELDS, admitted)
    binary = [
        {
            "schema_version": "1",
            "feature_set": feature_set,
            "binary_bytes": "1",
            "base64_ng_symbol_count": "0",
            "method": "nm-and-file-size",
        }
        for feature_set in sorted(VALIDATOR.BINARY_FEATURE_SETS)
    ]
    write_csv(
        directory / "binary-resources.csv",
        VALIDATOR.BINARY_RESOURCE_FIELDS,
        binary,
    )
    write_manifest(directory, env)


def write_manifest(directory: Path, env: dict[str, object]) -> None:
    source = env["source"]
    measurement = env["measurement"]
    if not isinstance(source, dict) or not isinstance(measurement, dict):
        raise SystemExit("performance evidence tests: invalid manifest fixture")
    metadata = {
        "source_commit": source["commit"],
        "source_status": source["status"],
        "campaign_id": measurement["campaign_id"],
        "sample_count": measurement["sample_count"],
        "target_bytes_per_sample": measurement["target_bytes_per_sample"],
        **VALIDATOR.MANIFEST_STATIC_METADATA,
    }
    lines = ["base64-ng performance evidence schema 1"]
    lines.extend(f"{key}={value}" for key, value in metadata.items())
    lines.append("artifacts:")
    for artifact in sorted(VALIDATOR.MANIFEST_ARTIFACTS):
        digest = hashlib.sha256((directory / artifact).read_bytes()).hexdigest()
        lines.append(f"{digest}  {artifact}")
    (directory / "MANIFEST.txt").write_text(
        "\n".join(lines) + "\n", encoding="utf-8"
    )


def test_exact_csv_and_complete_bundle() -> None:
    with tempfile.TemporaryDirectory(prefix="base64-ng-perf-bundle-") as temporary:
        directory = Path(temporary)
        write_bundle(directory)
        VALIDATOR.load_validated_bundle(directory)
        VALIDATOR.validate_derived(directory)
        VALIDATOR.validate_manifest(directory)

        raw = directory / "raw-run-1.csv"
        lines = raw.read_text(encoding="utf-8").splitlines()
        lines[1] += ",surplus"
        raw.write_text("\n".join(lines) + "\n", encoding="utf-8")
        try:
            VALIDATOR.load_validated_bundle(directory)
        except SystemExit as error:
            if "expected 19 fields, found 20" not in str(error):
                raise
        else:
            raise SystemExit("performance evidence tests: surplus cell was accepted")

        write_bundle(directory)
        rows = benchmark_rows()
        write_csv(
            directory / "raw-run-1.csv",
            VALIDATOR.BENCHMARK_FIELDS,
            rows[:-2],
        )
        try:
            VALIDATOR.load_validated_bundle(directory)
        except SystemExit as error:
            if "incomplete matrix" not in str(error):
                raise
        else:
            raise SystemExit(
                "performance evidence tests: partial standalone bundle was accepted"
            )

        write_bundle(directory)
        binary = directory / "binary-resources.csv"
        text = binary.read_text(encoding="utf-8")
        binary.write_text(text.replace("nm-and-file-size", "=1+1", 1), encoding="utf-8")
        try:
            VALIDATOR.validate_derived(directory)
        except SystemExit as error:
            if "method must match" not in str(error):
                raise
        else:
            raise SystemExit(
                "performance evidence tests: formula-like derived field was accepted"
            )


def test_manifest_contract() -> None:
    with tempfile.TemporaryDirectory(prefix="base64-ng-perf-manifest-") as temporary:
        directory = Path(temporary)
        manifest = directory / "MANIFEST.txt"
        metadata_mutations = {
            "source_commit": "f" * 40,
            "source_status": "dirty",
            "campaign_id": "other-campaign",
            "sample_count": "999",
            "target_bytes_per_sample": "999",
        }
        for key, value in metadata_mutations.items():
            write_bundle(directory)
            lines = manifest.read_text(encoding="utf-8").splitlines()
            manifest.write_text(
                "\n".join(
                    f"{key}={value}" if line.startswith(f"{key}=") else line
                    for line in lines
                )
                + "\n",
                encoding="utf-8",
            )
            try:
                VALIDATOR.validate_derived(directory)
            except SystemExit as error:
                if "metadata mismatch" not in str(error):
                    raise
            else:
                raise SystemExit(
                    f"performance evidence tests: mismatched {key} was accepted"
                )

        write_bundle(directory)
        lines = manifest.read_text(encoding="utf-8").splitlines()
        manifest.write_text(
            "\n".join(line for line in lines if not line.endswith("  summary.csv"))
            + "\n",
            encoding="utf-8",
        )
        try:
            VALIDATOR.validate_derived(directory)
        except SystemExit as error:
            if "artifact inventory mismatch" not in str(error):
                raise
        else:
            raise SystemExit(
                "performance evidence tests: incomplete manifest was accepted"
            )

        write_bundle(directory)
        environment_path = directory / "environment.json"
        environment_path.write_text(
            environment_path.read_text(encoding="utf-8") + "\n", encoding="utf-8"
        )
        try:
            VALIDATOR.validate_derived(directory)
        except SystemExit as error:
            if "checksum mismatch" not in str(error):
                raise
        else:
            raise SystemExit(
                "performance evidence tests: changed artifact was accepted"
            )


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="base64-ng-perf-valid-") as temporary:
        directory = Path(temporary)
        write_fixture(
            directory,
            environment(),
            availability_rows(),
            resource_rows(),
            benchmark_rows(),
        )
        validate_fixture(directory)

    expect_failure(
        lambda env, _availability, _resources, _benchmarks: env["source"].update(
            {"status": "dirty"}
        ),
        "not captured from a clean tree",
    )
    expect_failure(
        lambda env, _availability, _resources, _benchmarks: env["source"].update(
            {"commit": "short"}
        ),
        "no full source commit",
    )
    expect_failure(
        lambda _env, _availability, _resources, benchmarks: benchmarks.__delitem__(
            slice(-2, None)
        ),
        "incomplete matrix",
    )
    expect_failure(
        lambda _env, _availability, _resources, benchmarks: benchmarks[-1].update(
            {"sample_index": "2"}
        ),
        "sample indexes",
    )
    expect_failure(
        lambda env, _availability, _resources, _benchmarks: env[
            "measurement"
        ].update({"campaign_id": "=formula"}),
        "campaign_id must match",
    )
    expect_failure(
        lambda _env, _availability, resources, _benchmarks: resources[0].update(
            {"feature_set": "@formula"}
        ),
        "feature_set must match",
    )
    expect_failure(
        lambda _env, _availability, resources, _benchmarks: resources[0].update(
            {"name": "=1+1"}
        ),
        "name must match",
    )
    expect_failure(
        lambda _env, _availability, resources, _benchmarks: resources.pop(),
        "incomplete or unexpected resource inventory",
    )
    expect_failure(
        lambda _env, _availability, _resources, benchmarks: benchmarks[0].update(
            {"iterations": "1"}
        ),
        "iterations do not match campaign target",
    )
    expect_failure(
        lambda _env, _availability, _resources, benchmarks: benchmarks[0].update(
            {"encoded_len": "999"}
        ),
        "incorrect encoded length",
    )
    expect_failure(
        lambda _env, _availability, _resources, benchmarks: benchmarks[0].update(
            {"throughput_mib_s": "999999999999.000000"}
        ),
        "throughput does not match raw timing fields",
    )
    test_capture_requires_clean_commit()
    test_exact_csv_and_complete_bundle()
    test_manifest_contract()
    print("performance evidence mutation tests: ok")


if __name__ == "__main__":
    main()
