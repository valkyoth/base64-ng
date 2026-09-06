#!/usr/bin/env python3
"""Tests for the per-crate release policy helper."""

from __future__ import annotations

import copy
import contextlib
import importlib.util
import io
import json
import tempfile
from pathlib import Path
from types import SimpleNamespace


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
        "version": "1.0.10",
        "crates": {
            name: {
                "previous_version": "1.0.9",
                "version": "1.0.9",
                "change": "unchanged",
                "publish": False,
                "reason": "test",
            }
            for name in release_crates.PUBLISH_ORDER
        },
    }


def base_packages() -> dict[str, dict]:
    packages = {
        name: package(name, "1.0.9") for name in release_crates.PUBLISH_ORDER
    }
    packages["base64-ng-sanitization"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-derive"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-mime"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-imap"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-multibase"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-password"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-openpgp"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-pem"]["dependencies"] = [{"name": "base64-ng"}]
    packages["base64-ng-serde"]["dependencies"] = [
        {"name": "base64-ng"},
        {"name": "serde"},
    ]
    packages["base64-ng-bytes"]["dependencies"] = [
        {"name": "base64-ng"},
        {"name": "bytes"},
    ]
    packages["base64-ng-subtle"]["dependencies"] = [
        {"name": "base64-ng"},
        {"name": "subtle"},
    ]
    packages["base64-ng-tokio"]["dependencies"] = [
        {"name": "base64-ng"},
        {"name": "tokio"},
    ]
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
        "version must be 1.0.10",
        release_crates.validate_plan_entry,
        "base64-ng",
        plan["crates"]["base64-ng"],
        "1.0.10",
    )


def test_dependency_only_changes_must_patch_bump() -> None:
    entry = {
        "previous_version": "1.0.9",
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


def test_metadata_only_changes_must_patch_bump_and_publish() -> None:
    entry = {
        "previous_version": "2.0.3",
        "version": "2.0.4",
        "change": "metadata",
        "publish": True,
        "reason": "release metadata",
    }
    release_crates.validate_plan_entry(
        "base64-ng", entry, "2.0.4", "selective-patch"
    )

    entry["publish"] = False
    assert_fails(
        "changed under selective-patch but publish is false",
        release_crates.validate_plan_entry,
        "base64-ng",
        entry,
        "2.0.4",
        "selective-patch",
    )


def test_unchanged_crates_are_not_published() -> None:
    entry = {
        "previous_version": "1.0.9",
        "version": "1.0.9",
        "change": "unchanged",
        "publish": True,
        "reason": "test",
    }
    assert_fails(
        "unchanged but publish is true",
        release_crates.validate_plan_entry,
        "base64-ng",
        entry,
        "1.0.10",
    )


def test_publish_plan_skips_unchanged_crates() -> None:
    plan = base_plan()
    plan["crates"]["base64-ng"] = {
        "previous_version": "1.0.9",
        "version": "1.0.10",
        "change": "code",
        "publish": True,
        "reason": "test",
    }
    assert release_crates.publish_plan(plan) == ("base64-ng",)


def test_development_policy_requires_synced_unpublished_versions() -> None:
    entry = {
        "previous_version": "1.3.9",
        "version": "2.0.0",
        "change": "code",
        "publish": False,
        "reason": "development",
    }
    release_crates.validate_plan_entry(
        "base64-ng", entry, "2.0.0", "development-blocked"
    )

    entry["publish"] = True
    assert_fails(
        "cannot publish",
        release_crates.validate_plan_entry,
        "base64-ng",
        entry,
        "2.0.0",
        "development-blocked",
    )


def test_synced_family_requires_every_changed_crate_to_publish() -> None:
    entry = {
        "previous_version": "1.3.9",
        "version": "2.0.0",
        "change": "code",
        "publish": True,
        "reason": "release candidate",
    }
    release_crates.validate_plan_entry(
        "base64-ng", entry, "2.0.0", "synced-family"
    )

    entry["publish"] = False
    assert_fails(
        "code changes but publish is false",
        release_crates.validate_plan_entry,
        "base64-ng",
        entry,
        "2.0.0",
        "synced-family",
    )


def test_selective_patch_accepts_changed_and_unchanged_crates() -> None:
    changed = {
        "previous_version": "2.0.3",
        "version": "2.0.4",
        "change": "dependency",
        "publish": True,
        "reason": "dependency refresh",
    }
    unchanged = {
        "previous_version": "2.0.3",
        "version": "2.0.3",
        "change": "unchanged",
        "publish": False,
        "reason": "no package change",
    }
    release_crates.validate_plan_entry(
        "base64-ng-sanitization", changed, "2.0.4", "selective-patch"
    )
    release_crates.validate_plan_entry(
        "base64-ng-bytes", unchanged, "2.0.4", "selective-patch"
    )


def test_selective_patch_rejects_changed_crate_at_another_version() -> None:
    entry = {
        "previous_version": "2.0.2",
        "version": "2.0.3",
        "change": "dependency",
        "publish": True,
        "reason": "stale patch",
    }
    assert_fails(
        "selected for the selective patch",
        release_crates.validate_plan_entry,
        "base64-ng-sanitization",
        entry,
        "2.0.4",
        "selective-patch",
    )


def test_selective_patch_rejects_non_patch_code_transitions() -> None:
    for previous, release in (
        ("2.0.3", "2.1.0"),
        ("2.0.3", "3.0.0"),
        ("2.0.3", "2.0.2"),
    ):
        entry = {
            "previous_version": previous,
            "version": release,
            "change": "code",
            "publish": True,
            "reason": "invalid selective transition",
        }
        assert_fails(
            "must increase the patch version on the same major/minor line",
            release_crates.validate_plan_entry,
            "base64-ng",
            entry,
            release,
            "selective-patch",
        )


def test_selective_patch_requires_published_core() -> None:
    plan = base_plan()
    assert_fails(
        "must publish base64-ng",
        release_crates.validate_release_policy,
        plan["crates"],
        "1.0.10",
        "selective-patch",
    )

    plan["crates"]["base64-ng"] = {
        "previous_version": "1.0.9",
        "version": "1.0.10",
        "change": "dependency",
        "publish": True,
        "reason": "maintenance",
    }
    release_crates.validate_release_policy(
        plan["crates"], "1.0.10", "selective-patch"
    )


def test_npm_plan_supports_independent_versions() -> None:
    unchanged = {
        "name": "@valkyoth/base64-ng-wasm-loader",
        "previous_version": "2.0.3",
        "version": "2.0.3",
        "change": "unchanged",
        "publish": False,
        "reason": "no package change",
    }
    release_crates.validate_npm_plan(unchanged)
    assert release_crates.npm_plan_output(
        {"version": "2.0.5", "npm": unchanged}
    ) == (
        "release=2.0.5\nname=@valkyoth/base64-ng-wasm-loader\n"
        "version=2.0.3\npublish=false"
    )

    changed = dict(unchanged)
    changed.update(version="2.0.4", change="code", publish=True)
    release_crates.validate_npm_plan(changed)


def test_npm_plan_rejects_inconsistent_selection() -> None:
    entry = {
        "name": "@valkyoth/base64-ng-wasm-loader",
        "previous_version": "2.0.3",
        "version": "2.0.3",
        "change": "unchanged",
        "publish": True,
        "reason": "invalid selection",
    }
    assert_fails(
        "unchanged npm package cannot be selected",
        release_crates.validate_npm_plan,
        entry,
    )

    entry.update(version="2.0.4", change="code", publish=False)
    assert_fails(
        "changed npm package must be selected",
        release_crates.validate_npm_plan,
        entry,
    )

    entry.update(version="2.0.2", publish=True)
    assert_fails(
        "must increase its independent version",
        release_crates.validate_npm_plan,
        entry,
    )


def test_npm_manifest_and_lock_must_match_plan() -> None:
    plan = {
        "npm": {
            "name": "@valkyoth/base64-ng-wasm-loader",
            "version": "2.0.4",
        }
    }
    original_root = release_crates.ROOT
    try:
        with tempfile.TemporaryDirectory() as raw:
            release_crates.ROOT = Path(raw)
            package_dir = (
                release_crates.ROOT / "packages" / "base64-ng-wasm-loader"
            )
            package_dir.mkdir(parents=True)
            package = {
                "name": plan["npm"]["name"],
                "version": plan["npm"]["version"],
            }
            (package_dir / "package.json").write_text(json.dumps(package))
            lock = {**package, "packages": {"": package}}
            (package_dir / "package-lock.json").write_text(json.dumps(lock))
            release_crates.verify_npm_package(plan)

            lock["packages"][""]["version"] = "2.0.3"
            (package_dir / "package-lock.json").write_text(json.dumps(lock))
            assert_fails(
                "npm locked package does not match",
                release_crates.verify_npm_package,
                plan,
            )
    finally:
        release_crates.ROOT = original_root


def test_publish_sequence_dry_runs_dependents_after_index_wait() -> None:
    plan = base_plan()
    plan["crates"]["base64-ng"]["version"] = "1.0.10"
    plan["crates"]["base64-ng-serde"]["version"] = "1.0.10"
    events: list[str] = []

    original_publish_dry_run = release_crates.publish_dry_run
    original_publish = release_crates.publish
    original_wait_for_index = release_crates.wait_for_index
    try:
        release_crates.publish_dry_run = lambda package, args: events.append(
            f"dry-run {package}"
        )
        release_crates.publish = lambda package, args: events.append(
            f"publish {package}"
        )
        release_crates.wait_for_index = lambda package, version, dry_run: events.append(
            f"wait {package} {version}"
        )

        release_crates.publish_sequence(
            SimpleNamespace(skip_checks=False, dry_run=False),
            ("base64-ng", "base64-ng-serde"),
            plan,
        )
    finally:
        release_crates.publish_dry_run = original_publish_dry_run
        release_crates.publish = original_publish
        release_crates.wait_for_index = original_wait_for_index

    assert events == [
        "dry-run base64-ng",
        "publish base64-ng",
        "wait base64-ng 1.0.10",
        "dry-run base64-ng-serde",
        "publish base64-ng-serde",
    ]


def test_post_tag_full_gate_uses_candidate_mode() -> None:
    calls: list[tuple[str, ...]] = []
    original_run = release_crates.run
    try:
        release_crates.run = lambda command, dry_run: calls.append(tuple(command))
        release_crates.run_preflight(
            SimpleNamespace(
                skip_checks=False,
                version="1.0.10",
                full_gate=True,
                dry_run=False,
            ),
            ("base64-ng",),
        )
    finally:
        release_crates.run = original_run

    assert calls == [("scripts/stable_release_gate.sh", "candidate")]


def test_release_tag_check_requires_valid_signature() -> None:
    calls: list[tuple[str, ...]] = []

    original_try_capture = release_crates.try_capture
    original_run = release_crates.subprocess.run
    try:
        release_crates.try_capture = lambda command: {
            ("git", "rev-parse", "HEAD"): "abc",
            ("git", "rev-list", "-n", "1", "v1.0.10"): "abc",
        }.get(tuple(command))

        def fake_run(command, **kwargs):
            calls.append(tuple(command))
            return SimpleNamespace(returncode=0, stdout="Good signature", stderr="")

        release_crates.subprocess.run = fake_run
        with contextlib.redirect_stdout(io.StringIO()):
            release_crates.check_release_tag("1.0.10", require_tag=True)
    finally:
        release_crates.try_capture = original_try_capture
        release_crates.subprocess.run = original_run

    assert (
        str(release_crates.ROOT / "scripts" / "verify-release-tag.sh"),
        "v1.0.10",
    ) in calls


def test_release_tag_check_rejects_unverified_required_tag() -> None:
    original_try_capture = release_crates.try_capture
    original_run = release_crates.subprocess.run
    try:
        release_crates.try_capture = lambda command: {
            ("git", "rev-parse", "HEAD"): "abc",
            ("git", "rev-list", "-n", "1", "v1.0.10"): "abc",
        }.get(tuple(command))

        def fake_run(command, **kwargs):
            return SimpleNamespace(returncode=1, stdout="", stderr="no signature")

        release_crates.subprocess.run = fake_run
        try:
            with contextlib.redirect_stderr(io.StringIO()):
                release_crates.check_release_tag("1.0.10", require_tag=True)
        except SystemExit as exc:
            assert exc.code == 1
            return
    finally:
        release_crates.try_capture = original_try_capture
        release_crates.subprocess.run = original_run

    raise AssertionError("expected release tag check to exit")


def run_tests() -> None:
    tests = (
        test_current_plan_accepts_unchanged_crates,
        test_code_changes_must_use_milestone_version,
        test_dependency_only_changes_must_patch_bump,
        test_metadata_only_changes_must_patch_bump_and_publish,
        test_unchanged_crates_are_not_published,
        test_publish_plan_skips_unchanged_crates,
        test_development_policy_requires_synced_unpublished_versions,
        test_synced_family_requires_every_changed_crate_to_publish,
        test_selective_patch_accepts_changed_and_unchanged_crates,
        test_selective_patch_rejects_changed_crate_at_another_version,
        test_selective_patch_rejects_non_patch_code_transitions,
        test_selective_patch_requires_published_core,
        test_npm_plan_supports_independent_versions,
        test_npm_plan_rejects_inconsistent_selection,
        test_npm_manifest_and_lock_must_match_plan,
        test_publish_sequence_dry_runs_dependents_after_index_wait,
        test_post_tag_full_gate_uses_candidate_mode,
        test_release_tag_check_requires_valid_signature,
        test_release_tag_check_rejects_unverified_required_tag,
    )
    for test in tests:
        test()


if __name__ == "__main__":
    run_tests()
