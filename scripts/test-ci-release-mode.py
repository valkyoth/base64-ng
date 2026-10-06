#!/usr/bin/env python3
"""Prevent exhaustive release suites from silently returning to push CI."""

from pathlib import Path
import shlex
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[1]
GUARD = "${{ github.event_name == 'workflow_dispatch' && inputs.full_release }}"
HEAVY = {"development-checks", "big-endian-qemu", "riscv-qemu", "sve-qemu"}


def validate_python_install(workflow):
    step = next(step for step in workflow["jobs"]["checks"]["steps"]
                if step.get("name") == "Check release-only CI routing")
    commands = step["run"].replace("\\\n", "").splitlines()
    # Force reinstall so a preinstalled distribution cannot bypass hash checking.
    assert shlex.split(commands[0]) == [
        "python3", "-m", "pip", "install", "--disable-pip-version-check",
        "--require-hashes", "--only-binary=:all:", "--no-deps", "--force-reinstall",
        "-r", "ci/requirements-ci.txt",
    ]
    assert commands[1:] == ["python3 scripts/test-ci-release-mode.py"]


def validate(workflow):
    # BaseLoader preserves GitHub's `on` key instead of YAML 1.1 boolean coercion.
    triggers = workflow["on"]
    assert set(triggers) == {"push", "pull_request", "workflow_dispatch"}
    option = triggers["workflow_dispatch"]["inputs"]["full_release"]
    assert option["type"] == "boolean" and option["default"] == "false"
    for name, job in workflow["jobs"].items():
        if name in HEAVY:
            assert job["if"] == GUARD, name
        else:
            assert "if" not in job, name
            needs = job.get("needs", [])
            assert not HEAVY.intersection([needs] if isinstance(needs, str) else needs), name
    assert HEAVY <= workflow["jobs"].keys()
    commands = {name: "\n".join(step.get("run", "") for step in job["steps"])
                for name, job in workflow["jobs"].items()}
    assert "scripts/checks.sh --core" in commands["checks"]
    for name, command in {
        "development-checks": "scripts/checks.sh --development",
        "big-endian-qemu": "scripts/check_big_endian_qemu.sh --all",
        "riscv-qemu": "scripts/check_riscv_qemu.sh",
        "sve-qemu": "scripts/check_sve_qemu.sh",
    }.items():
        assert command in commands[name]
        assert all(command not in value for other, value in commands.items() if other != name)


class ReleaseModeTests(unittest.TestCase):
    def load(self):
        return yaml.load((ROOT / ".github/workflows/ci.yml").read_text(), Loader=yaml.BaseLoader)

    def test_actual_workflow_keeps_full_suites_opt_in(self):
        validate(self.load())

    def test_windows_push_gate_excludes_native_campaign(self):
        steps = self.load()["jobs"]["platform"]["steps"]
        step = next(s for s in steps if s["name"] == "Check Windows public APIs and companion forwarding")
        self.assertEqual(step["if"], "matrix.os == 'windows-latest'")
        self.assertEqual(step["shell"], "pwsh")
        self.assertEqual(step["run"].splitlines(),
                         ["./scripts/test-windows-gate.ps1", "python scripts/test-windows-measurement.py",
                          "./scripts/check_windows.ps1"])

    def test_ci_installs_only_hash_approved_wheels(self):
        validate_python_install(self.load())

    def test_insecure_install_mutations_are_rejected(self):
        for flag in ("--require-hashes", "--only-binary=:all:", "--no-deps", "--force-reinstall"):
            with self.subTest(flag=flag):
                workflow = self.load()
                step = next(s for s in workflow["jobs"]["checks"]["steps"]
                            if s.get("name") == "Check release-only CI routing")
                step["run"] = step["run"].replace(flag, "")
                with self.assertRaises(AssertionError):
                    validate_python_install(workflow)

    def test_missing_or_broadened_guards_are_rejected(self):
        for job in HEAVY:
            for guard in (None, "true", "${{ inputs.full_release }}"):
                with self.subTest(job=job, guard=guard):
                    workflow = self.load()
                    workflow["jobs"][job]["if"] = guard
                    with self.assertRaises(AssertionError):
                        validate(workflow)

    def test_default_enabled_and_scheduled_runs_are_rejected(self):
        workflow = self.load()
        workflow["on"]["workflow_dispatch"]["inputs"]["full_release"]["default"] = "true"
        with self.assertRaises(AssertionError):
            validate(workflow)
        workflow = self.load()
        workflow["on"]["schedule"] = [{"cron": "0 0 * * *"}]
        with self.assertRaises(AssertionError):
            validate(workflow)


if __name__ == "__main__":
    unittest.main()
