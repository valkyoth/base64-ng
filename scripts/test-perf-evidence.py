#!/usr/bin/env python3
"""Mutation tests for the performance-evidence trust boundary."""

from __future__ import annotations

import csv
import importlib.util
import json
import subprocess
import tempfile
from copy import deepcopy
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
            "name": f"{category}-value",
            "feature_set": feature_set,
            "value": "1",
            "unit": "bytes",
            "method": "test",
        }
        for category in (
            "stack-bound",
            "adapter-pending-memory",
            "adapter-size",
        )
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
                                "encoded_len": input_len,
                                "iterations": "1",
                                "elapsed_ns": "1",
                                "throughput_mib_s": "1.0",
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

        output = directory.parent / f"{directory.name}-environment.json"
        subprocess.run(
            [str(CAPTURE_PATH), str(output)], cwd=directory, check=True
        )
        captured = json.loads(output.read_text(encoding="utf-8"))
        output.unlink()
        if captured["source"]["status"] != "clean":
            raise SystemExit("performance evidence tests: clean capture was not clean")
        if len(captured["source"]["commit"]) != 40:
            raise SystemExit("performance evidence tests: capture did not use full commit")

        (directory / "untracked").write_text("dirty\n", encoding="utf-8")
        result = subprocess.run(
            [str(CAPTURE_PATH), str(output)],
            cwd=directory,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode == 0 or "dirty tree" not in result.stderr:
            raise SystemExit("performance evidence tests: dirty capture was accepted")


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
    test_capture_requires_clean_commit()
    print("performance evidence mutation tests: ok")


if __name__ == "__main__":
    main()
