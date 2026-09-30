#!/usr/bin/env python3
"""Offline mutation/entry-point tests for blocked development releases."""

import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

import release_crates as release


ROOT = Path(__file__).resolve().parents[1]


def reject(message, function, *args):
    try:
        function(*args)
    except RuntimeError as error:
        assert message in str(error), str(error)
    else:
        raise AssertionError(f"accepted invalid fixture: {message}")


def write_plan(path, plan):
    lines = []
    for section, values in [("release", plan["release"]), ("npm", plan["npm"])]:
        lines.append(f"[{section}]")
        lines.extend(f"{key} = {json.dumps(value)}" for key, value in values.items())
    for name, values in plan["crates"].items():
        lines.append(f'[crates."{name}"]')
        lines.extend(f"{key} = {json.dumps(value)}" for key, value in values.items())
    path.write_text("\n".join(lines) + "\n")


def main():
    entry = dict(previous_version="2.0.4", version="2.1.0", change="metadata",
                 publish=False, reason="development")
    plan = {
        "release": {"version": "2.1.0", "policy": "development-blocked"},
        "crates": {name: dict(entry) for name in release.PUBLISH_ORDER},
        "npm": dict(entry, name="@valkyoth/base64-ng-wasm-loader"),
    }
    packages = {name: dict(name=name, version="2.1.0", dependencies=[], publish=[])
                for name in release.PUBLISH_ORDER}
    with tempfile.TemporaryDirectory(prefix="base64-ng-development-") as raw:
        root = Path(raw)
        scripts = root / "scripts"
        scripts.mkdir()
        for name in ("release_crates.py", "release_wasm_loader.sh",
                     "stable_release_gate.sh", "validate-release-readiness.sh"):
            shutil.copy2(ROOT / "scripts" / name, scripts / name)
        path = root / "release-crates.toml"
        write_plan(path, plan)
        parsed = release.release_plan(path)
        release.verify_publish_order(packages, parsed)
        assert release.publish_plan(parsed) == ()
        assert "publish=false" in release.npm_plan_output(parsed)

        for section in ("npm", *release.PUBLISH_ORDER):
            changed = copy.deepcopy(plan)
            target = changed["npm"] if section == "npm" else changed["crates"][section]
            target["publish"] = True
            write_plan(path, changed)
            reject("development-blocked", release.release_plan, path)
            target["publish"] = False
            target["version"] = "2.0.4"
            write_plan(path, changed)
            reject("development version", release.release_plan, path)
        write_plan(path, plan)

        changed_packages = copy.deepcopy(packages)
        changed_packages["base64-ng"]["publish"] = None
        reject("publish = false", release.verify_publish_order, changed_packages, parsed)

        package_dir = root / "packages/base64-ng-wasm-loader"
        package_dir.mkdir(parents=True)
        package = dict(name=plan["npm"]["name"], version="2.1.0", private=True)
        lock = dict(name=package["name"], version="2.1.0", packages={"": package})
        (package_dir / "package-lock.json").write_text(json.dumps(lock))
        (package_dir / "package.json").write_text(json.dumps(package))
        original_root = release.ROOT
        try:
            release.ROOT = root
            release.verify_npm_package(parsed)
            package["private"] = False
            (package_dir / "package.json").write_text(json.dumps(package))
            reject("must be private", release.verify_npm_package, parsed)
            package["private"] = True
            (package_dir / "package.json").write_text(json.dumps(package))
        finally:
            release.ROOT = original_root

        metadata = dict(packages=[dict(value, id=name) for name, value in packages.items()],
                        workspace_members=list(packages))
        (root / "metadata.json").write_text(json.dumps(metadata))
        (root / "Cargo.toml").write_text('[package]\nversion = "2.1.0"\n')
        bin_dir = root / "bin"
        bin_dir.mkdir()
        # Any unexpected build/publish command fails rather than reaching a registry.
        commands = {
            "cargo": '#!/bin/sh\n[ "$1" = metadata ] || exit 90\ncat metadata.json\n',
            "node": '#!/bin/sh\ncase "$*" in *publishConfig*) echo public;; '
                    '*.name*) echo @valkyoth/base64-ng-wasm-loader;; '
                    '*.version*) echo 2.1.0;; *) exit 90;; esac\n',
            "npm": '#!/bin/sh\nexit 90\n',
        }
        for name, contents in commands.items():
            command = bin_dir / name
            command.write_text(contents)
            command.chmod(0o755)
        env = dict(os.environ, PATH=f"{bin_dir}{os.pathsep}{os.environ['PATH']}")
        env.pop("BASE64_NG_EVIDENCE_SIGNING_KEY", None)
        cases = [
            ([sys.executable, str(scripts / "release_crates.py"), "--yes",
              "--skip-checks", "--allow-dirty", "--allow-untagged"], "development-blocked"),
            ([sys.executable, str(scripts / "release_crates.py"), "--dry-run"],
             "development-blocked"),
            ([str(scripts / "release_wasm_loader.sh"), "publish"], "not selected"),
            ([str(scripts / "release_wasm_loader.sh"), "dry-run"], "not selected"),
            ([str(scripts / "stable_release_gate.sh"), "candidate"], "development-blocked"),
            ([str(scripts / "stable_release_gate.sh"), "release"], "development-blocked"),
            ([str(scripts / "validate-release-readiness.sh"), "v2.1.0"], "blocked"),
        ]
        for command, expected in cases:
            result = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True)
            assert result.returncode == 1, (command, result.stdout, result.stderr)
            assert expected in result.stderr, (command, result.stderr)
    print("development release: version mutations and all publication entry points fail closed")


if __name__ == "__main__":
    main()
