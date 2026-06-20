#!/usr/bin/env python3
"""Publish base64-ng workspace crates in crates.io dependency order."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - release host guard.
    print("Python 3.11+ is required because this script uses tomllib.", file=sys.stderr)
    raise


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PLAN = ROOT / "release-crates.toml"
CHANGE_KINDS = ("code", "dependency", "unchanged")

PUBLISH_ORDER = (
    "base64-ng",
    "base64-ng-sanitization",
    "base64-ng-derive",
    "base64-ng-serde",
    "base64-ng-bytes",
    "base64-ng-subtle",
    "base64-ng-tokio",
)


def run(command: list[str], *, dry_run: bool) -> None:
    print(f"+ {' '.join(command)}", flush=True)
    if dry_run:
        return
    subprocess.run(command, cwd=ROOT, check=True)


def capture(command: list[str]) -> str:
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def try_capture(command: list[str]) -> str | None:
    result = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    if result.returncode != 0:
        return None
    return result.stdout.strip()


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def parse_version(version: str) -> tuple[int, int, int]:
    parts = version.split(".")
    if len(parts) != 3:
        raise RuntimeError(f"version must be MAJOR.MINOR.PATCH: {version}")
    try:
        major, minor, patch = (int(part) for part in parts)
    except ValueError as exc:
        raise RuntimeError(f"version must be numeric: {version}") from exc
    return (major, minor, patch)


def release_version(plan_path: Path = DEFAULT_PLAN) -> str:
    plan = load_toml(plan_path)
    return plan["release"]["version"]


def cargo_metadata() -> dict:
    raw = capture(["cargo", "metadata", "--format-version", "1", "--no-deps"])
    return json.loads(raw)


def workspace_packages(metadata: dict) -> dict[str, dict]:
    workspace_ids = set(metadata["workspace_members"])
    return {
        package["name"]: package
        for package in metadata["packages"]
        if package["id"] in workspace_ids
    }


def release_plan(plan_path: Path) -> dict:
    plan = load_toml(plan_path)
    release = plan.get("release", {})
    crates = plan.get("crates", {})
    version = release.get("version")
    if not isinstance(version, str):
        raise RuntimeError("release-crates.toml is missing [release].version")
    if set(crates) != set(PUBLISH_ORDER):
        raise RuntimeError(
            "release-crates.toml crates are not in sync with PUBLISH_ORDER: "
            f"expected {tuple(sorted(PUBLISH_ORDER))}, actual {tuple(sorted(crates))}"
        )
    parse_version(version)
    for package_name, entry in crates.items():
        validate_plan_entry(package_name, entry, version)
    return {"version": version, "crates": crates}


def validate_plan_entry(package_name: str, entry: dict, release: str) -> None:
    previous = entry.get("previous_version")
    version = entry.get("version")
    change = entry.get("change")
    publish = entry.get("publish")
    reason = entry.get("reason")
    if not all(isinstance(value, str) for value in (previous, version, change, reason)):
        raise RuntimeError(f"{package_name} has incomplete release plan metadata")
    if change not in CHANGE_KINDS:
        raise RuntimeError(f"{package_name} has invalid change kind {change!r}")
    if not isinstance(publish, bool):
        raise RuntimeError(f"{package_name} publish must be true or false")

    previous_version = parse_version(previous)
    planned_version = parse_version(version)
    release_parts = parse_version(release)

    if change == "code":
        if planned_version != release_parts:
            raise RuntimeError(
                f"{package_name} has code changes, so version must be {release}"
            )
        if not publish:
            raise RuntimeError(f"{package_name} has code changes but publish is false")
    elif change == "dependency":
        same_line = planned_version[:2] == previous_version[:2]
        patch_bump = planned_version[2] > previous_version[2]
        if not same_line or not patch_bump:
            raise RuntimeError(
                f"{package_name} dependency-only bumps must stay on the existing "
                "minor line and increase only the patch number"
            )
        if not publish:
            raise RuntimeError(
                f"{package_name} has dependency-only changes but publish is false"
            )
    else:
        if planned_version != previous_version:
            raise RuntimeError(
                f"{package_name} is unchanged but version differs from previous_version"
            )
        if publish:
            raise RuntimeError(f"{package_name} is unchanged but publish is true")


def require_clean_tree(*, allow_dirty: bool) -> None:
    if allow_dirty:
        return

    status = capture(["git", "status", "--porcelain"])
    if status:
        print("Refusing to publish from a dirty worktree:", file=sys.stderr)
        print(status, file=sys.stderr)
        print("Commit or stash changes, or pass --allow-dirty.", file=sys.stderr)
        sys.exit(1)


def verify_publish_order(packages: dict[str, dict], plan: dict) -> None:
    package_names = tuple(packages)
    expected_names = tuple(sorted(PUBLISH_ORDER))
    actual_names = tuple(sorted(package_names))
    if actual_names != expected_names:
        raise RuntimeError(
            "release_crates.py PUBLISH_ORDER is not in sync with workspace "
            f"packages: expected {expected_names}, actual {actual_names}"
        )

    seen: set[str] = set()
    for package_name in PUBLISH_ORDER:
        package = packages[package_name]
        planned_version = plan["crates"][package_name]["version"]
        if package["version"] != planned_version:
            raise RuntimeError(
                f"{package_name} is version {package['version']}, "
                f"expected {planned_version}"
            )

        for dependency in package["dependencies"]:
            dependency_name = dependency["name"]
            if dependency_name in packages and dependency_name not in seen:
                raise RuntimeError(
                    f"{package_name} depends on {dependency_name}, but "
                    f"{dependency_name} appears later in PUBLISH_ORDER"
                )
        seen.add(package_name)


def check_release_tag(version: str, *, require_tag: bool) -> None:
    tag = f"v{version}"
    head = try_capture(["git", "rev-parse", "HEAD"])
    tagged_commit = try_capture(["git", "rev-list", "-n", "1", tag])
    if head is None or tagged_commit is None:
        message = f"release tag {tag!r} was not found"
        if require_tag:
            print(f"Refusing to publish: {message}.", file=sys.stderr)
            sys.exit(1)
        print(f"Warning: {message}.", file=sys.stderr)
        return

    if head != tagged_commit:
        message = f"HEAD is not tagged as {tag} (HEAD {head}, {tag} {tagged_commit})"
        if require_tag:
            print(f"Refusing to publish: {message}.", file=sys.stderr)
            sys.exit(1)
        print(f"Warning: {message}.", file=sys.stderr)
        return

    print(f"Release tag {tag} points at HEAD.")


def confirm_no_verify(args: argparse.Namespace) -> int:
    if not args.no_verify or args.dry_run:
        return 0

    print(
        "\nWARNING: --no-verify bypasses cargo package verification.\n"
        "Use it only with a documented release incident or crates.io issue.\n"
        "Type 'no-verify confirmed' to continue:",
        file=sys.stderr,
    )
    response = input().strip()
    if response != "no-verify confirmed":
        print("Aborted.", file=sys.stderr)
        return 1
    return 0


def publish_dry_run(package: str, args: argparse.Namespace) -> None:
    command = ["cargo", "publish", "-p", package, "--dry-run"]
    if args.allow_dirty:
        command.append("--allow-dirty")
    run(command, dry_run=args.dry_run)


def run_preflight(args: argparse.Namespace, steps: tuple[str, ...]) -> None:
    if args.skip_checks:
        print("Skipping preflight checks by request.")
        return

    parse_version(args.version)
    if args.full_gate:
        run(["scripts/stable_release_gate.sh", "release"], dry_run=args.dry_run)
        return

    run(["scripts/checks.sh"], dry_run=args.dry_run)


def publish_plan(plan: dict) -> tuple[str, ...]:
    return tuple(
        package
        for package in PUBLISH_ORDER
        if plan["crates"][package]["publish"]
    )


def selected_steps(start_at: str, steps: tuple[str, ...]) -> tuple[str, ...]:
    if not steps:
        return ()
    try:
        index = steps.index(start_at)
    except ValueError as exc:
        raise RuntimeError(f"unknown package for --start-at: {start_at}") from exc
    return steps[index:]


def wait_for_index(package: str, version: str, *, dry_run: bool) -> None:
    print()
    print(f"Published {package} {version}.")
    print(f"Wait until crates.io shows: https://crates.io/crates/{package}/{version}")
    print("Then press Enter to continue with dependent crates.")
    if dry_run:
        print("[dry-run] skipping wait")
        return
    input()
    time.sleep(5)


def publish(package: str, args: argparse.Namespace) -> None:
    command = ["cargo", "publish", "-p", package]
    if args.allow_dirty:
        command.append("--allow-dirty")
    if args.no_verify:
        command.append("--no-verify")
    run(command, dry_run=args.dry_run)


def publish_sequence(args: argparse.Namespace, steps: tuple[str, ...], plan: dict) -> None:
    for index, package in enumerate(steps):
        if not args.skip_checks:
            publish_dry_run(package, args)
        publish(package, args)
        version = plan["crates"][package]["version"]
        if index != len(steps) - 1:
            wait_for_index(package, version, dry_run=args.dry_run)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Publish base64-ng workspace crates in crates.io order."
    )
    parser.add_argument(
        "--version",
        default=None,
        help="Expected release version. Defaults to release-crates.toml.",
    )
    parser.add_argument(
        "--plan",
        default=str(DEFAULT_PLAN),
        help="Path to the per-crate release plan.",
    )
    parser.add_argument(
        "--start-at",
        default=None,
        choices=PUBLISH_ORDER,
        help="Resume publishing at a package if an earlier step already succeeded.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Validate publish order and versions, then exit.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print publish commands without running them or waiting.",
    )
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="Allow publishing from a dirty worktree and pass --allow-dirty to cargo.",
    )
    parser.add_argument(
        "--skip-checks",
        action="store_true",
        help="Skip local checks and cargo publish dry-runs before publishing.",
    )
    parser.add_argument(
        "--full-gate",
        action="store_true",
        help=(
            "Run scripts/stable_release_gate.sh release before publishing. "
            "By default, the publish helper assumes the full gate, including "
            "Kani, already passed before tagging."
        ),
    )
    parser.add_argument(
        "--no-verify",
        action="store_true",
        help="Pass --no-verify to cargo publish. Use only with a documented reason.",
    )
    parser.add_argument(
        "--require-tag",
        action="store_true",
        help="Refuse to continue unless HEAD matches the v<version> release tag.",
    )
    parser.add_argument(
        "--allow-untagged",
        action="store_true",
        help=(
            "Allow publishing from an untagged HEAD. Use only for a documented "
            "crates.io recovery incident."
        ),
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Do not ask for the initial confirmation.",
    )
    args = parser.parse_args()
    raw_plan_path = Path(args.plan)
    plan_path = (
        raw_plan_path
        if raw_plan_path.is_absolute()
        else (ROOT / raw_plan_path).resolve()
    )
    plan = release_plan(plan_path)
    if args.version is None:
        args.version = plan["version"]
    elif args.version != plan["version"]:
        print(
            f"Refusing to publish: --version {args.version} does not match "
            f"{plan_path.name} release {plan['version']}.",
            file=sys.stderr,
        )
        return 1

    metadata = cargo_metadata()
    packages = workspace_packages(metadata)
    verify_publish_order(packages, plan)

    if args.check:
        print("release_crates.py publish order is up to date.")
        print(f"release_crates.py release plan is {args.version}.")
        return 0

    require_clean_tree(allow_dirty=args.allow_dirty or args.dry_run)
    require_tag = args.require_tag or (not args.dry_run and not args.allow_untagged)
    check_release_tag(args.version, require_tag=require_tag)

    planned_publish = publish_plan(plan)
    start_at = args.start_at or (planned_publish[0] if planned_publish else "")
    steps = selected_steps(start_at, planned_publish)

    print(f"Workspace root: {ROOT}")
    print(f"Release version: {args.version}")
    print("Publish sequence:")
    if steps:
        for package in steps:
            version = plan["crates"][package]["version"]
            change = plan["crates"][package]["change"]
            print(f"  - {package} {version} ({change})")
    else:
        print("  - no crates selected for publishing")
    print()

    if not args.yes:
        answer = input("Type the release version to start publishing: ").strip()
        if answer != args.version:
            print("Version confirmation did not match; aborting.", file=sys.stderr)
            return 1

    no_verify_result = confirm_no_verify(args)
    if no_verify_result != 0:
        return no_verify_result

    run_preflight(args, steps)

    publish_sequence(args, steps, plan)

    print()
    print("Release publish sequence completed.")
    print(f"Recommended follow-up: cargo info base64-ng@{args.version}")
    print("If companion crates were published, also run:")
    print(f"  cargo info base64-ng-sanitization@{args.version}")
    print(f"  cargo info base64-ng-derive@{args.version}")
    print(f"  cargo info base64-ng-serde@{args.version}")
    print(f"  cargo info base64-ng-bytes@{args.version}")
    print(f"  cargo info base64-ng-subtle@{args.version}")
    print(f"  cargo info base64-ng-tokio@{args.version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
