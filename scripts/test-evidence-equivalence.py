#!/usr/bin/env python3
"""Mutation tests for metadata-only release evidence reuse."""

from __future__ import annotations

import hashlib
import pathlib
import shutil
import subprocess
import tempfile


SOURCE = pathlib.Path(__file__).resolve().with_name("evidence-equivalence.py")
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
    shutil.copy2(ALLOWLIST, repo / "security/evidence-reuse-allowlist.txt")
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
    )
    artifact_lines = []
    for directory in campaign_directories:
        artifact = repo / "target/release-evidence" / directory / "artifact.txt"
        artifact.parent.mkdir(parents=True)
        artifact.write_text(f"{directory}\n", encoding="utf-8")
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
    assert "policy=metadata-only-v1" in accepted.stdout

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

    allowlist = repo / "security/evidence-reuse-allowlist.txt"
    allowlist.write_text(allowlist.read_text(encoding="utf-8") + "src/lib.rs\n", encoding="utf-8")
    allowlist_drift = commit(repo, "allowlist drift")
    run(repo, "--evidence-commit", evidence, "--release-commit", allowlist_drift, succeeds=False)
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
