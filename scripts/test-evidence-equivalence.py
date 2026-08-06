#!/usr/bin/env python3
"""Mutation tests for metadata-only release evidence reuse."""

from __future__ import annotations

import hashlib
import pathlib
import shutil
import subprocess
import tempfile


SOURCE = pathlib.Path(__file__).resolve().with_name("evidence-equivalence.py")
VERIFIER = pathlib.Path(__file__).resolve().with_name(
    "verify-release-evidence-signature.sh"
)
ALLOWLIST = SOURCE.parents[1] / "security/evidence-reuse-allowlist.txt"


def run(repo: pathlib.Path, *arguments: str, succeeds: bool) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["python3", "scripts/evidence-equivalence.py", *arguments],
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if (result.returncode == 0) != succeeds:
        raise AssertionError(
            f"unexpected status {result.returncode}:\nstdout={result.stdout}\nstderr={result.stderr}"
        )
    return result


def commit(repo: pathlib.Path, message: str) -> str:
    subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", message], cwd=repo, check=True)
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()


with tempfile.TemporaryDirectory() as raw_temp:
    root = pathlib.Path(raw_temp)
    repo = root / "repo"
    (repo / "scripts").mkdir(parents=True)
    (repo / "security").mkdir()
    (repo / "src").mkdir()
    (repo / "docs").mkdir()
    shutil.copy2(SOURCE, repo / "scripts/evidence-equivalence.py")
    shutil.copy2(VERIFIER, repo / "scripts/verify-release-evidence-signature.sh")
    shutil.copy2(ALLOWLIST, repo / "security/evidence-reuse-allowlist.txt")
    key = root / "evidence-key"
    subprocess.run(
        ["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)],
        check=True,
    )
    fingerprint = subprocess.check_output(
        ["ssh-keygen", "-lf", f"{key}.pub", "-E", "sha256"], text=True
    ).split()[1]
    principal = "evidence-test@example.invalid"
    public_key = pathlib.Path(f"{key}.pub").read_text(encoding="utf-8").strip()
    (repo / "security/evidence-signers").write_text(
        f'{principal} namespaces="base64-ng-evidence-v2" {public_key}\n',
        encoding="utf-8",
    )
    verifier = repo / "scripts/verify-release-evidence-signature.sh"
    verifier.write_text(
        verifier.read_text(encoding="utf-8")
        .replace("base64-ng-evidence-signer-v2", principal)
        .replace(
            "SHA256:vf1eXq+UBZWKsX3DD1iakRK2Pk9AsXoJzZj/tNcdczc",
            fingerprint,
        ),
        encoding="utf-8",
    )
    verifier.chmod(0o755)
    (repo / "src/lib.rs").write_text("pub fn value() -> u8 { 1 }\n", encoding="utf-8")
    (repo / "Cargo.lock").write_text("# lock\n", encoding="utf-8")
    (repo / "docs/RELEASE.md").write_text("baseline\n", encoding="utf-8")
    (repo / ".gitignore").write_text("/target/\n", encoding="utf-8")
    subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.email", "evidence@example.invalid"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.name", "Evidence Test"], cwd=repo, check=True)
    evidence = commit(repo, "evidence")
    retained = repo / "target/release-evidence/FINAL-MANIFEST.txt"
    retained.parent.mkdir(parents=True)
    campaign_directories = (
        "miri",
        "2.0-memory-sanitizers",
        "fuzz",
        "dudect",
        "backend",
        "kani",
        "asm",
        "simd-asm",
        "rvv-asm",
        "sve-asm",
        "big-endian-qemu",
        "riscv-qemu",
        "sve-qemu",
    )
    artifact_lines = []
    for directory in campaign_directories:
        artifact_name = "report.txt" if directory.endswith("qemu") else "artifact.txt"
        artifact = repo / "target/release-evidence" / directory / artifact_name
        artifact.parent.mkdir(parents=True)
        if directory == "big-endian-qemu":
            contents = (
                f"source_commit={evidence}\n"
                "s390x_result=pass\n"
                "powerpc64_result=pass\n"
            )
        elif directory in {"riscv-qemu", "sve-qemu"}:
            contents = f"source_commit={evidence}\nresult=pass\n"
        else:
            contents = f"{directory}\n"
        artifact.write_text(contents, encoding="utf-8")
        digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
        artifact_lines.append(f"{digest}  {artifact.relative_to(repo).as_posix()}")
    retained.write_text(
        "source:\n"
        f"commit={evidence}\n"
        "tree_state=clean\n"
        "artifact-hashes:\n"
        + "\n".join(artifact_lines)
        + "\n",
        encoding="utf-8",
    )
    subprocess.run(
        [
            "ssh-keygen",
            "-Y",
            "sign",
            "-f",
            str(key),
            "-n",
            "base64-ng-evidence-v2",
            str(retained),
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    subprocess.run(["git", "clean", "-fdq"], cwd=repo, check=True)

    (repo / "docs/RELEASE.md").write_text("accepted metadata\n", encoding="utf-8")
    release = commit(repo, "metadata")
    # Keep the retained campaign manifest and artifacts unchanged.
    accepted = run(
        repo,
        "--evidence-commit",
        evidence,
        "--release-commit",
        release,
        "--retained-manifest",
        str(retained),
        succeeds=True,
    )
    assert "policy=metadata-only-v2" in accepted.stdout

    tampered_artifact = repo / "target/release-evidence/miri/artifact.txt"
    original_artifact = tampered_artifact.read_bytes()
    tampered_artifact.write_text("tampered\n", encoding="utf-8")
    run(
        repo,
        "--evidence-commit",
        evidence,
        "--retained-manifest",
        str(retained),
        succeeds=False,
    )
    tampered_artifact.write_bytes(original_artifact)

    for directory in campaign_directories:
        artifact_name = "report.txt" if directory.endswith("qemu") else "artifact.txt"
        artifact = repo / "target/release-evidence" / directory / artifact_name
        original = artifact.read_bytes()
        artifact.write_bytes(original + b"tampered\n")
        run(
            repo,
            "--evidence-commit",
            evidence,
            "--retained-manifest",
            str(retained),
            succeeds=False,
        )
        artifact.write_bytes(original)

    original_manifest = retained.read_bytes()
    original_manifest_signature = retained.with_suffix(retained.suffix + ".sig").read_bytes()
    semantic_mutations = {
        "big-endian-qemu": ("s390x_result=pass", "s390x_result=fail"),
        "riscv-qemu": (f"source_commit={evidence}", "source_commit=" + "0" * 40),
        "sve-qemu": ("result=pass", "result=fail"),
    }
    for directory, (before, after) in semantic_mutations.items():
        report = repo / "target/release-evidence" / directory / "report.txt"
        original_report = report.read_text(encoding="utf-8")
        report.write_text(original_report.replace(before, after), encoding="utf-8")
        relative = report.relative_to(repo).as_posix()
        new_digest = hashlib.sha256(report.read_bytes()).hexdigest()
        lines = retained.read_text(encoding="utf-8").splitlines()
        retained.write_text(
            "\n".join(
                f"{new_digest}  {relative}" if line.endswith(f"  {relative}") else line
                for line in lines
            )
            + "\n",
            encoding="utf-8",
        )
        signature_path = retained.with_suffix(retained.suffix + ".sig")
        signature_path.unlink()
        subprocess.run(
            [
                "ssh-keygen",
                "-Y",
                "sign",
                "-f",
                str(key),
                "-n",
                "base64-ng-evidence-v2",
                str(retained),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        run(
            repo,
            "--evidence-commit",
            evidence,
            "--retained-manifest",
            str(retained),
            succeeds=False,
        )
        report.write_text(original_report, encoding="utf-8")
        retained.write_bytes(original_manifest)
        signature_path.write_bytes(original_manifest_signature)

    signature = retained.with_suffix(retained.suffix + ".sig")
    original_signature = signature.read_bytes()
    signature.write_text("invalid signature\n", encoding="utf-8")
    run(
        repo,
        "--evidence-commit",
        evidence,
        "--retained-manifest",
        str(retained),
        succeeds=False,
    )
    signature.write_bytes(original_signature)

    allowlist = repo / "security/evidence-reuse-allowlist.txt"
    allowlist.write_text(allowlist.read_text(encoding="utf-8") + "src/lib.rs\n", encoding="utf-8")
    allowlist_drift = commit(repo, "allowlist drift")
    run(repo, "--evidence-commit", evidence, "--release-commit", allowlist_drift, succeeds=False)
    subprocess.run(["git", "reset", "--hard", "-q", release], cwd=repo, check=True)

    validator = repo / "scripts/evidence-equivalence.py"
    validator.write_text(validator.read_text(encoding="utf-8") + "\n# drift\n", encoding="utf-8")
    validator_drift = commit(repo, "validator drift")
    run(repo, "--evidence-commit", evidence, "--release-commit", validator_drift, succeeds=False)
    subprocess.run(["git", "reset", "--hard", "-q", release], cwd=repo, check=True)

    (repo / "src/lib.rs").write_text("pub fn value() -> u8 { 2 }\n", encoding="utf-8")
    runtime = commit(repo, "runtime")
    run(repo, "--evidence-commit", evidence, "--release-commit", runtime, succeeds=False)

    subprocess.run(["git", "reset", "--hard", "-q", release], cwd=repo, check=True)
    (repo / "docs/RELEASE.md").write_text("dirty\n", encoding="utf-8")
    run(repo, "--evidence-commit", evidence, succeeds=False)
    subprocess.run(["git", "reset", "--hard", "-q", release], cwd=repo, check=True)

    retained.write_text(
        "source:\n" f"commit={release}\n" "tree_state=clean\n",
        encoding="utf-8",
    )
    run(
        repo,
        "--evidence-commit",
        evidence,
        "--retained-manifest",
        str(retained),
        succeeds=False,
    )

print("evidence equivalence tests: ok")
