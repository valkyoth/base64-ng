#!/usr/bin/env python3
"""Tests for the per-crate release policy helper."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "release_crates.py"


def load_release_crates():
    spec = importlib.util.spec_from_file_location("release_crates", SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load release_crates.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


release_crates = load_release_crates()


def package(name: str, version: str, deps: tuple[str, ...] = ()) -> dict:
    return {
        "name": name,
        "version": version,
        "dependencies": [{"name": dep} for dep in deps],
    }


def base_plan() -> dict:
    return {
        "version": "1.0.9",
        "crates": {
            name: {
                "previous_version": "1.0.8",
                "version": "1.0.8",
                "change": "unchanged",
                "publish": False,
                "reason": "test",
            }
            for name in release_crates.PUBLISH_ORDER
        },
    }


def base_packages() -> dict[str, dict]:
    packages = {
        name: package(name, "1.0.8") for name in release_crates.PUBLISH_ORDER
    }
    packages["base64-ng-sanitization"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-derive"]["dependencies"] = [{"name": "base64-ng"}]
    return packages


def assert_fails(expected: str, func, *args) -> None:
    try:
        func(*args)
    except RuntimeError as exc:
        if expected not in str(exc):
            raise AssertionError(f"expected {expected!r} in {exc!r}") from exc
        return
    raise AssertionError("expected failure")


def test_current_plan_accepts_unchanged_crates() -> None:
    release_crates.verify_publish_order(base_packages(), base_plan())


def test_code_changes_must_use_milestone_version() -> None:
    plan = base_plan()
    plan["crates"]["base64-ng"]["change"] = "code"
    plan["crates"]["base64-ng"]["publish"] = True
    assert_fails(
        "version must be 1.0.9",
        release_crates.validate_plan_entry,
        "base64-ng",
        plan["crates"]["base64-ng"],
        "1.0.9",
    )


def test_dependency_only_changes_must_patch_bump() -> None:
    entry = {
        "previous_version": "1.0.8",
        "version": "1.1.0",
        "change": "dependency",
        "publish": True,
        "reason": "test",
    }
    assert_fails(
        "dependency-only bumps",
        release_crates.validate_plan_entry,
        "base64-ng",
        entry,
        "1.1.0",
    )


def test_unchanged_crates_are_not_published() -> None:
    entry = {
        "previous_version": "1.0.8",
        "version": "1.0.8",
        "change": "unchanged",
        "publish": True,
        "reason": "test",
    }
    assert_fails(
        "unchanged but publish is true",
        release_crates.validate_plan_entry,
        "base64-ng",
        entry,
        "1.0.9",
    )


def test_publish_plan_skips_unchanged_crates() -> None:
    plan = base_plan()
    plan["crates"]["base64-ng"] = {
        "previous_version": "1.0.8",
        "version": "1.0.9",
        "change": "code",
        "publish": True,
        "reason": "test",
    }
    assert release_crates.publish_plan(plan) == ("base64-ng",)


def run_tests() -> None:
    tests = (
        test_current_plan_accepts_unchanged_crates,
        test_code_changes_must_use_milestone_version,
        test_dependency_only_changes_must_patch_bump,
        test_unchanged_crates_are_not_published,
        test_publish_plan_skips_unchanged_crates,
    )
    for test in tests:
        test()


if __name__ == "__main__":
    run_tests()
