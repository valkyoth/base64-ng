#!/usr/bin/env python3
"""Prove that retained expensive evidence still covers a release commit."""

from __future__ import annotations

import argparse
import hashlib
import pathlib
import re
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_ALLOWLIST = ROOT / "security/evidence-reuse-allowlist.txt"
FULL_COMMIT = re.compile(r"[0-9a-f]{40}")
HASH_LINE = re.compile(r"([0-9a-f]{64})  (target/release-evidence/.+)")
PERMITTED_METADATA_PATHS = {
    "2.0.0-release-plan.md",
    "README.md",
    "docs/2.0_RELEASE_FREEZE.md",
    "docs/RELEASE.md",
    "docs/RELEASE_EVIDENCE.md",
    "security/pentest/v2.0.0.md",
}
RETAINED_CAMPAIGN_PREFIXES = (
    "target/release-evidence/miri/",
    "target/release-evidence/2.0-memory-sanitizers/",
    "target/release-evidence/fuzz/",
    "target/release-evidence/dudect/",
    "target/release-evidence/backend/",
    "target/release-evidence/kani/",
    "target/release-evidence/asm/",
    "target/release-evidence/simd-asm/",
    "target/release-evidence/rvv-asm/",
    "target/release-evidence/sve-asm/",
    "target/release-evidence/big-endian-qemu/",
    "target/release-evidence/riscv-qemu/",
    "target/release-evidence/sve-qemu/",
)


def fail(message: str) -> None:
    raise SystemExit(f"evidence equivalence: {message}")


def git(*arguments: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        fail(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout.strip()


def resolve_commit(revision: str, label: str) -> str:
    commit = git("rev-parse", "--verify", f"{revision}^{{commit}}")
    if not FULL_COMMIT.fullmatch(commit):
        fail(f"{label} is not a full Git commit: {commit}")
    return commit


def read_allowlist(path: pathlib.Path) -> tuple[list[str], str]:
    try:
        raw = path.read_bytes()
    except OSError as error:
        fail(f"cannot read allowlist {path}: {error}")
    entries: list[str] = []
    for raw_line in raw.decode("utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("/") or "*" in line or "?" in line or line.endswith("/"):
            fail(f"allowlist entry must be an exact relative path: {line}")
        entries.append(line)
    if not entries or len(entries) != len(set(entries)):
        fail("allowlist is empty or contains duplicate entries")
    if set(entries) != PERMITTED_METADATA_PATHS:
        fail("allowlist does not match the reviewed metadata-path policy")
    return entries, hashlib.sha256(raw).hexdigest()


def require_clean_release(release_commit: str) -> None:
    head = resolve_commit("HEAD", "HEAD")
    if release_commit != head:
        return
    status = git("status", "--porcelain", "--untracked-files=all")
    if status:
        fail("release HEAD has a dirty worktree")


def require_exact_key(path: pathlib.Path, key: str, expected: str) -> None:
    values = [
        line.removeprefix(f"{key}=")
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.startswith(f"{key}=")
    ]
    if values != [expected]:
        fail(f"retained report has no singleton {key}={expected}: {path}")


def require_qemu_reports(evidence_commit: str) -> None:
    big_endian = ROOT / "target/release-evidence/big-endian-qemu/report.txt"
    riscv = ROOT / "target/release-evidence/riscv-qemu/report.txt"
    sve = ROOT / "target/release-evidence/sve-qemu/report.txt"
    for report in (big_endian, riscv, sve):
        if not report.is_file():
            fail(f"retained QEMU report is missing: {report.relative_to(ROOT)}")
        require_exact_key(report, "source_commit", evidence_commit)
    require_exact_key(big_endian, "s390x_result", "pass")
    require_exact_key(big_endian, "powerpc64_result", "pass")
    require_exact_key(riscv, "result", "pass")
    require_exact_key(sve, "result", "pass")


def require_manifest_source(
    path: pathlib.Path,
    signature: pathlib.Path,
    evidence_commit: str,
) -> None:
    verifier = ROOT / "scripts/verify-release-evidence-signature.sh"
    result = subprocess.run(
        [str(verifier), str(path), str(signature)],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        fail(f"retained FINAL-MANIFEST signature is invalid: {detail}")
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        fail(f"cannot read retained evidence manifest {path}: {error}")
    commits = [line.removeprefix("commit=") for line in lines if line.startswith("commit=")]
    states = [line.removeprefix("tree_state=") for line in lines if line.startswith("tree_state=")]
    if commits != [evidence_commit] or states != ["clean"]:
        fail("retained FINAL-MANIFEST is not bound to the clean evidence commit")

    retained_hashes: dict[str, str] = {}
    for line in lines:
        match = HASH_LINE.fullmatch(line)
        if match is not None:
            retained_hashes[match.group(2)] = match.group(1)
    for prefix in RETAINED_CAMPAIGN_PREFIXES:
        recorded = {path: digest for path, digest in retained_hashes.items() if path.startswith(prefix)}
        if not recorded:
            fail(f"retained FINAL-MANIFEST lacks campaign artifacts under {prefix}")
        current_files = {
            file.relative_to(ROOT).as_posix()
            for file in (ROOT / prefix).glob("**/*")
            if file.is_file()
        }
        if current_files != set(recorded):
            fail(f"retained campaign artifact inventory changed under {prefix}")
        for relative_path, expected in recorded.items():
            actual = hashlib.sha256((ROOT / relative_path).read_bytes()).hexdigest()
            if actual != expected:
                fail(f"retained campaign artifact checksum changed: {relative_path}")
    require_qemu_reports(evidence_commit)


def protected_listing(commit: str, allowlist: set[str]) -> str:
    listing = git("ls-tree", "-r", "--full-tree", commit)
    protected: list[str] = []
    for line in listing.splitlines():
        try:
            metadata, path = line.split("\t", 1)
        except ValueError:
            fail("unexpected git ls-tree output")
        if path not in allowlist:
            protected.append(f"{metadata}\t{path}")
    return "\n".join(protected) + "\n"


def validate(
    evidence_revision: str,
    release_revision: str,
    allowlist_path: pathlib.Path,
    retained_manifest: pathlib.Path | None,
    retained_signature: pathlib.Path | None,
) -> dict[str, object]:
    evidence_commit = resolve_commit(evidence_revision, "evidence revision")
    release_commit = resolve_commit(release_revision, "release revision")
    if evidence_commit == release_commit:
        fail("evidence reuse requires two distinct commits")

    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", evidence_commit, release_commit],
        cwd=ROOT,
        check=False,
    )
    if ancestor.returncode != 0:
        fail("evidence commit is not an ancestor of the release commit")
    merges = git("rev-list", "--merges", f"{evidence_commit}..{release_commit}")
    if merges:
        fail("evidence reuse range contains a merge commit")

    require_clean_release(release_commit)
    entries, allowlist_hash = read_allowlist(allowlist_path)
    allowed = set(entries)
    changed = git(
        "diff",
        "--name-only",
        "--diff-filter=ACDMRTUXB",
        evidence_commit,
        release_commit,
    ).splitlines()
    if not changed:
        fail("evidence reuse range contains no changes")
    unexpected = sorted(set(changed) - allowed)
    if unexpected:
        fail("non-metadata paths changed: " + ", ".join(unexpected))

    evidence_protected = protected_listing(evidence_commit, allowed)
    release_protected = protected_listing(release_commit, allowed)
    if evidence_protected != release_protected:
        fail("protected repository contents differ despite the path allowlist")
    protected_hash = hashlib.sha256(evidence_protected.encode("utf-8")).hexdigest()

    if retained_manifest is not None:
        signature = retained_signature or retained_manifest.with_suffix(
            retained_manifest.suffix + ".sig"
        )
        require_manifest_source(retained_manifest, signature, evidence_commit)

    return {
        "evidence_commit": evidence_commit,
        "release_commit": release_commit,
        "allowlist_hash": allowlist_hash,
        "protected_hash": protected_hash,
        "changed": changed,
    }


def render(result: dict[str, object]) -> str:
    changed = result["changed"]
    assert isinstance(changed, list)
    lines = [
        "base64-ng release evidence equivalence",
        "",
        "policy=metadata-only-v2",
        f"evidence_commit={result['evidence_commit']}",
        f"release_commit={result['release_commit']}",
        f"allowlist_sha256={result['allowlist_hash']}",
        f"protected_tree_sha256={result['protected_hash']}",
        "package_evidence=must-be-regenerated",
        "changed_paths:",
        *changed,
        "",
    ]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-commit", required=True)
    parser.add_argument("--release-commit", default="HEAD")
    parser.add_argument("--allowlist", type=pathlib.Path, default=DEFAULT_ALLOWLIST)
    parser.add_argument("--retained-manifest", type=pathlib.Path)
    parser.add_argument("--retained-signature", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    arguments = parser.parse_args()

    result = validate(
        arguments.evidence_commit,
        arguments.release_commit,
        arguments.allowlist,
        arguments.retained_manifest,
        arguments.retained_signature,
    )
    output = render(result)
    if arguments.output is None:
        sys.stdout.write(output)
        return
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = arguments.output.with_name(f".{arguments.output.name}.tmp")
    temporary.write_text(output, encoding="utf-8")
    temporary.replace(arguments.output)
    print(f"evidence equivalence: wrote {arguments.output}")


if __name__ == "__main__":
    main()
