#!/usr/bin/env python3
"""Mutation tests for the narrowly scoped external-campaign reuse policy."""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts" / "validate-campaign-source-equivalence.py"
SPEC = importlib.util.spec_from_file_location("campaign_source_validator", VALIDATOR)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR_MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = VALIDATOR_MODULE
SPEC.loader.exec_module(VALIDATOR_MODULE)


def git(repo: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def commit(repo: Path, path: str, content: str, message: str) -> str:
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    git(repo, "add", path)
    git(repo, "commit", "-m", message)
    return git(repo, "rev-parse", "HEAD")


def configure_signing(repo: Path) -> None:
    key = repo / ".git/fixture-key"
    subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)], check=True)
    (repo / "security").mkdir(exist_ok=True)
    (repo / "security/release-signers").write_text(
        'fixture@example.invalid namespaces="git" ' + key.with_suffix(".pub").read_text())
    git(repo, "config", "gpg.format", "ssh")
    git(repo, "config", "user.signingkey", str(key))
    git(repo, "config", "commit.gpgsign", "true")
    git(repo, "add", "security/release-signers")


def run(repo: Path, campaign: str, success: bool, reason: str | None = None) -> None:
    original = VALIDATOR_MODULE.ROOT
    VALIDATOR_MODULE.ROOT = repo
    try:
        try:
            VALIDATOR_MODULE.validate(campaign, "HEAD")
        except SystemExit as error:
            if success:
                raise
            if reason is not None and reason not in str(error):
                raise AssertionError(f"wrong rejection: {error}; expected {reason}") from error
        else:
            if not success:
                raise SystemExit("campaign source equivalence mutation passed")
    finally:
        VALIDATOR_MODULE.ROOT = original


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        repo = Path(directory) / "repo"
        repo.mkdir()
        git(repo, "init", "-q")
        git(repo, "config", "user.name", "fixture")
        git(repo, "config", "user.email", "fixture@example.invalid")
        campaign = commit(repo, "src/lib.rs", "pub fn value() {}\n", "campaign")
        report = "security/pentest/v2.0.0.md"
        for path in sorted(VALIDATOR_MODULE.ALLOWED_TOOLING_CHANGES - {report}):
            target = repo / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(f"fixture {path}\n", encoding="utf-8")
        git(repo, "add", ".")
        git(repo, "commit", "-m", "reviewed tooling correction")

        commit(repo, report, "final reviewed report\n", "report-only release commit")
        run(repo, campaign, True)

        (repo / "src/lib.rs").write_text("pub fn changed() {}\n", encoding="utf-8")
        git(repo, "add", "src/lib.rs")
        git(repo, "commit", "-m", "runtime mutation")
        run(repo, campaign, False)

    test_native_inventory_correction()
    test_final_manifest_sources()
    test_retained_runtime_provenance()
    print("campaign source equivalence: mutation checks ok")


def test_native_inventory_correction() -> None:
    module = VALIDATOR_MODULE
    original_pin = (module.FROZEN_21_COMMIT, module.FROZEN_21_TREE,
                    module.NEON_21_CORRECTION_COMMIT, module.NEON_21_HARDENING_COMMIT,
                    module.NEON_21_FIXTURE_COMMIT)
    mutations = (
        "valid", "dirty", "wrong-tree", "wrong-pin", "missing-tool",
        "alternate-correction", "unsigned-correction", "unsigned-policy", "wrong-signer",
        "alternate-policy",
        "unsigned-hardening", "alternate-hardening", "unsigned-fixture", "alternate-fixture",
        "extra-fixture-file", "reverted-fixture", "wrong-order",
        "missing-policy-anchor", "wrong-policy-anchor", "validator-drift", "split-correction",
        "second-correction", "reverted-runtime", "src/lib.rs", "Cargo.lock",
        "Cargo.toml", "rust-toolchain.toml", "fuzz/fuzz_targets/decode.rs",
        "tests/rfc4648.rs", "scripts/unchecked.py", "security/evidence-reuse-allowlist.txt",
        "crates/base64-ng-bytes/src/lib.rs", "portability/windows_native/src/abi.rs",
    )
    try:
        for mutation in mutations:
            with tempfile.TemporaryDirectory() as directory:
                repo = Path(directory)
                git(repo, "init", "-q")
                git(repo, "config", "user.name", "fixture")
                git(repo, "config", "user.email", "fixture@example.invalid")
                configure_signing(repo)
                campaign = commit(repo, "src/lib.rs", "original\n", "campaign")
                module.FROZEN_21_COMMIT = campaign
                module.FROZEN_21_TREE = git(repo, "rev-parse", "HEAD^{tree}")
                commit(repo, "docs/RELEASE_FREEZE_2.1.md", "evidence progress\n", "progress")
                paths = sorted(module.NEON_21_CORRECTION)
                if mutation == "missing-tool":
                    paths.pop()
                for path in paths:
                    file = repo / path
                    file.parent.mkdir(parents=True, exist_ok=True)
                    file.write_text("reviewed correction\n")
                if mutation == "split-correction":
                    git(repo, "add", paths[0])
                    git(repo, "commit", "-m", "first part of split correction")
                git(repo, "add", ".")
                git(repo, "-c", "commit.gpgsign=" + str(mutation != "unsigned-correction").lower(),
                    "commit", "-m", "native inventory correction")
                correction = git(repo, "rev-parse", "HEAD")
                module.NEON_21_CORRECTION_COMMIT = correction
                if mutation == "alternate-correction":
                    # Same permitted path set does not authorize a different commit.
                    git(repo, "commit", "--amend", "-m", "unapproved alternative")
                for path in module.NEON_21_HARDENING:
                    file = repo / path
                    file.parent.mkdir(parents=True, exist_ok=True)
                    file.write_text("reviewed inventory hardening\n")
                git(repo, "add", ".")
                git(repo, "-c", "commit.gpgsign=" + str(mutation != "unsigned-hardening").lower(),
                    "commit", "-m", "inventory hardening")
                module.NEON_21_HARDENING_COMMIT = git(repo, "rev-parse", "HEAD")
                if mutation == "alternate-hardening":
                    git(repo, "commit", "--amend", "-m", "unapproved hardening alternative")
                for path in module.NEON_21_FIXTURE:
                    (repo / path).write_text("exact-source NEON fixture\n")
                if mutation == "extra-fixture-file":
                    (repo / module.POLICY_PATH).write_text("unapproved validator edit\n")
                git(repo, "add", ".")
                git(repo, "-c", "commit.gpgsign=" + str(mutation != "unsigned-fixture").lower(),
                    "commit", "-m", "CI fixture correction")
                module.NEON_21_FIXTURE_COMMIT = git(repo, "rev-parse", "HEAD")
                if mutation == "alternate-fixture":
                    git(repo, "commit", "--amend", "-m", "unapproved fixture alternative")
                if mutation == "reverted-fixture":
                    fixture_path = next(iter(module.NEON_21_FIXTURE))
                    commit(repo, fixture_path, "changed\n", "unreviewed fixture edit")
                    commit(repo, fixture_path, "exact-source NEON fixture\n", "revert fixture edit")
                if mutation == "wrong-order":
                    module.NEON_21_HARDENING_COMMIT, module.NEON_21_FIXTURE_COMMIT = (
                        module.NEON_21_FIXTURE_COMMIT, module.NEON_21_HARDENING_COMMIT)
                for path in module.NEON_21_POLICY:
                    file = repo / path
                    file.parent.mkdir(parents=True, exist_ok=True)
                    file.write_bytes(VALIDATOR.read_bytes() if path == module.POLICY_PATH
                                     else b"reviewed policy hardening\n")
                if mutation == "wrong-signer":
                    other_key = repo / ".git/other-key"
                    subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f",
                                    str(other_key)], check=True)
                    git(repo, "config", "user.signingkey", str(other_key))
                git(repo, "add", ".")
                git(repo, "-c", "commit.gpgsign=" + str(mutation != "unsigned-policy").lower(),
                    "commit", "-m", "independently reviewed policy")
                policy = git(repo, "rev-parse", "HEAD")
                if mutation == "alternate-policy":
                    git(repo, "commit", "--amend", "-m", "unapproved policy alternative")
                if mutation == "dirty":
                    (repo / "src/lib.rs").write_text("dirty\n")
                elif mutation == "wrong-tree":
                    module.FROZEN_21_TREE = "0" * 40
                elif mutation == "wrong-pin":
                    module.FROZEN_21_COMMIT = "0" * 40
                elif mutation == "second-correction":
                    commit(repo, paths[0], "second edit\n", "unreviewed follow-up")
                elif mutation == "validator-drift":
                    commit(repo, module.POLICY_PATH, "arbitrary replacement\n", "replace verifier")
                elif mutation == "reverted-runtime":
                    commit(repo, "src/lib.rs", "changed\n", "runtime change")
                    commit(repo, "src/lib.rs", "original\n", "runtime revert")
                elif "/" in mutation or mutation.endswith((".lock", ".toml")):
                    commit(repo, mutation, "changed\n", "protected mutation")
                anchor = "" if mutation == "missing-policy-anchor" else (
                    correction if mutation == "wrong-policy-anchor" else policy)
                with patch.dict(os.environ, {module.POLICY_ENV: anchor}), patch.object(
                        module, "__file__", str(repo / module.POLICY_PATH)):
                    reasons = {
                        "unsigned-correction": "lacks an authorized signature",
                        "unsigned-policy": "lacks an authorized signature",
                        "unsigned-hardening": "lacks an authorized signature",
                        "unsigned-fixture": "lacks an authorized signature",
                        "wrong-signer": "lacks an authorized signature",
                        "missing-policy-anchor": "independently reviewed policy commit",
                        "wrong-policy-anchor": "executing validator differs",
                        "validator-drift": "executing validator differs",
                        "alternate-correction": "identity or inventory is not approved",
                        "alternate-policy": "identity or inventory is not approved",
                        "alternate-hardening": "identity or inventory is not approved",
                        "alternate-fixture": "identity or inventory is not approved",
                        "extra-fixture-file": "identity or inventory is not approved",
                        "reverted-fixture": "identity or inventory is not approved",
                        "wrong-order": "identity or inventory is not approved",
                        "split-correction": "identity or inventory is not approved",
                    }
                    run(repo, campaign, mutation == "valid", reasons.get(mutation))
    finally:
        (module.FROZEN_21_COMMIT, module.FROZEN_21_TREE, module.NEON_21_CORRECTION_COMMIT,
         module.NEON_21_HARDENING_COMMIT, module.NEON_21_FIXTURE_COMMIT) = original_pin


def test_final_manifest_sources() -> None:
    """Exercise the real finalizer/source policy; campaign outcomes have their own suite."""
    with tempfile.TemporaryDirectory() as directory:
        repo = Path(directory)
        scripts = repo / "scripts"
        scripts.mkdir()
        git(repo, "init", "-q")
        git(repo, "config", "user.name", "fixture")
        git(repo, "config", "user.email", "fixture@example.invalid")
        configure_signing(repo)
        (repo / ".gitignore").write_text("target/\n")
        (repo / "Cargo.toml").write_text('version = "2.1.0"\n')
        (repo / "Cargo.lock").write_text("fixture lock\n")
        (scripts / "evidence-source.sh").write_bytes((ROOT / "scripts/evidence-source.sh").read_bytes())
        rvv = scripts / "validate-rvv-admission-bundle.py"
        rvv.write_text("#!/bin/sh\nexit 0\n")
        rvv.chmod(0o755)
        git(repo, "add", ".")
        git(repo, "commit", "-m", "campaign fixture")
        campaign = git(repo, "rev-parse", "HEAD")
        tree = git(repo, "rev-parse", "HEAD^{tree}")
        for name in VALIDATOR_MODULE.NEON_21_CORRECTION:
            (repo / name).write_text("fixture correction\n")
        for name in ("finalize-release-evidence.sh", "validate-campaign-source-equivalence.py"):
            text = (ROOT / "scripts" / name).read_text()
            text = text.replace(VALIDATOR_MODULE.FROZEN_21_COMMIT, campaign)
            text = text.replace(VALIDATOR_MODULE.FROZEN_21_TREE, tree)
            (scripts / name).write_text(text)
        outcomes = scripts / "validate-release-evidence-outcomes.sh"
        outcomes.write_text("#!/bin/sh\nexit 0\n")
        outcomes.chmod(0o755)
        git(repo, "add", ".")
        git(repo, "commit", "-m", "complete correction fixture")
        correction = git(repo, "rev-parse", "HEAD")
        for name in VALIDATOR_MODULE.NEON_21_HARDENING:
            (repo / name).write_text("hardened fixture\n")
        git(repo, "add", ".")
        git(repo, "commit", "-m", "inventory hardening fixture")
        hardening = git(repo, "rev-parse", "HEAD")
        for name in VALIDATOR_MODULE.NEON_21_FIXTURE:
            (repo / name).write_text("exact-source NEON fixture\n")
        git(repo, "add", ".")
        git(repo, "commit", "-m", "CI fixture correction")
        fixture = git(repo, "rev-parse", "HEAD")
        for name in VALIDATOR_MODULE.NEON_21_POLICY:
            path = repo / name
            if name == VALIDATOR_MODULE.POLICY_PATH:
                text = VALIDATOR.read_text()
                for old, new in (
                    (VALIDATOR_MODULE.FROZEN_21_COMMIT, campaign),
                    (VALIDATOR_MODULE.FROZEN_21_TREE, tree),
                    (VALIDATOR_MODULE.NEON_21_CORRECTION_COMMIT, correction),
                    (VALIDATOR_MODULE.NEON_21_HARDENING_COMMIT, hardening),
                    (VALIDATOR_MODULE.NEON_21_FIXTURE_COMMIT, fixture),
                ):
                    text = text.replace(old, new)
                path.write_text(text)
            else:
                path.write_text("reviewed fixture-binding policy\n")
        git(repo, "add", ".")
        git(repo, "commit", "-m", "separately approved policy fixture")
        candidate = git(repo, "rev-parse", "HEAD")
        evidence = repo / "target/release-evidence"

        def write(name, text):
            file = evidence / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text(text)
            return file

        def source(commit):
            return f"source:\ncommit={commit}\ntree_state=clean\n"

        manifests = ["miri", "2.0-memory-sanitizers", "dudect", "backend", "asm",
                     "simd-asm", "neon-asm", "rvv-asm", "sve-asm", "wasm-simd", "fuzz"]
        for name in manifests:
            write(name + "/MANIFEST.txt", source(campaign))
        for name in ("normal", "advanced"):
            write("kani/" + name + "/source.txt", source(campaign))
        write("commit-53/MANIFEST.txt", source(candidate) +
              "neon_automatic_dispatch=exact-campaign-native-performance\n" +
              f"neon_source_commit={campaign}\n")
        for name in ("riscv-qemu", "sve-qemu"):
            write(name + "/report.txt", f"source_commit={campaign}\nresult=pass\n")
        write("big-endian-qemu/report.txt", f"source_commit={campaign}\n"
              "s390x_result=pass\npowerpc64_result=pass\n")
        write("riscv-native-admission/MANIFEST.txt", f"source_commit={campaign}\n"
              "execution_environment=real-hardware\nadmission_scope=linux-rvv-1.0-vlen256-spacemit-x60\n")
        write("sbom-MANIFEST.txt", source(candidate))
        write("reproducible/MANIFEST.txt", source(candidate))
        for name in ("spdx", "cyclonedx"):
            write(f"base64-ng.{name}.json", "{}\n")
        env = {key: value for key, value in os.environ.items() if not key.startswith("BASE64_NG_")}
        env["BASE64_NG_CAMPAIGN_SOURCE_COMMIT"] = campaign
        env[VALIDATOR_MODULE.POLICY_ENV] = candidate

        def finalize(success):
            result = subprocess.run(["sh", "scripts/finalize-release-evidence.sh"],
                                    cwd=repo, env=env, capture_output=True, text=True, timeout=30)
            if (result.returncode == 0) != success:
                raise AssertionError(result.stdout + result.stderr)

        finalize(True)
        index = (evidence / "FINAL-MANIFEST.txt").read_text()
        if f"runtime_campaign_commit={campaign}" not in index or f"release_commit={candidate}" not in index:
            raise AssertionError("Final manifest lost campaign/candidate distinction")
        for name, old, new in (
            ("miri/MANIFEST.txt", campaign, "0" * 40),
            ("miri/MANIFEST.txt", "tree_state=clean", "tree_state=dirty-development-only"),
            ("miri/MANIFEST.txt", "source:\n", "source:\nsource:\n"),
            ("fuzz/MANIFEST.txt", campaign, candidate),
            ("riscv-qemu/report.txt", campaign, candidate),
            ("sve-qemu/report.txt", "result=pass", "result=fail"),
            ("sbom-MANIFEST.txt", candidate, campaign),
            ("reproducible/MANIFEST.txt", candidate, campaign),
            ("commit-53/MANIFEST.txt", "neon_source_commit=" + campaign, "neon_source_commit=" + candidate),
            ("riscv-native-admission/MANIFEST.txt", campaign, candidate),
        ):
            file = evidence / name
            original = file.read_text()
            file.write_text(original.replace(old, new))
            finalize(False)
            file.write_text(original)
        finalize(True)


def test_retained_runtime_provenance() -> None:
    spec = importlib.util.spec_from_file_location("retention", ROOT / "scripts/evidence-equivalence.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    evidence = "1" * 40
    frozen = VALIDATOR_MODULE.FROZEN_21_COMMIT
    if module.retained_runtime_campaign([], evidence) != evidence:
        raise AssertionError("Legacy provenance changed")
    for lines in (["runtime_campaign_commit=bad"], ["runtime_campaign_commit=" + "0" * 40],
                  ["runtime_campaign_commit=" + frozen] * 2):
        try:
            module.retained_runtime_campaign(lines, evidence)
        except SystemExit:
            pass
        else:
            raise AssertionError("Invalid retained provenance accepted")
    for exit_code in (0, 1):
        with patch.object(module.subprocess, "run", return_value=subprocess.CompletedProcess(
                [], exit_code, "", "fixture rejection")) as command:
            try:
                actual = module.retained_runtime_campaign(["runtime_campaign_commit=" + frozen], evidence)
            except SystemExit:
                if exit_code == 0:
                    raise
            else:
                if exit_code != 0 or actual != frozen:
                    raise AssertionError("Incorrect retained campaign decision")
            if command.call_args.args[0][-4:] != ["--campaign", frozen, "--candidate", evidence]:
                raise AssertionError("Retained correction did not verify the signed evidence candidate")


if __name__ == "__main__":
    main()
