#!/usr/bin/env sh
set -eu

root="$(pwd)"
workdir="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-evidence-source-test.XXXXXX")"
trap 'rm -rf "$workdir"' EXIT INT TERM
repo="$workdir/repo"
mkdir -p "$repo"

git -C "$repo" init --quiet
git -C "$repo" config user.name "base64-ng evidence test"
git -C "$repo" config user.email "evidence-test@example.invalid"
printf 'lock\n' >"$repo/Cargo.lock"
printf 'source\n' >"$repo/source.rs"
git -C "$repo" add Cargo.lock source.rs
git -C "$repo" commit --quiet -m baseline

(
    cd "$repo"
    . "$root/scripts/evidence-source.sh"
    evidence_capture_source "evidence source test"
    [ "$EVIDENCE_TREE_STATE" = "clean" ]
    evidence_verify_source "evidence source test"
)

printf 'dirty\n' >>"$repo/source.rs"
if (
    cd "$repo"
    . "$root/scripts/evidence-source.sh"
    evidence_capture_source "evidence source test"
) >"$workdir/dirty.log" 2>&1
then
    echo "evidence source test: dirty tree unexpectedly passed strict capture" >&2
    exit 1
fi
grep -F -q 'refusing to generate release evidence from a dirty tree' "$workdir/dirty.log"

(
    cd "$repo"
    BASE64_NG_ALLOW_DIRTY_EVIDENCE=1
    export BASE64_NG_ALLOW_DIRTY_EVIDENCE
    . "$root/scripts/evidence-source.sh"
    evidence_capture_source "evidence source test"
    [ "$EVIDENCE_TREE_STATE" = "dirty-development-only" ]
    evidence_verify_source "evidence source test"
)

if (
    cd "$repo"
    BASE64_NG_ALLOW_DIRTY_EVIDENCE=1
    export BASE64_NG_ALLOW_DIRTY_EVIDENCE
    . "$root/scripts/evidence-source.sh"
    evidence_capture_source "evidence source test"
    printf 'changed after capture\n' >post-capture.txt
    evidence_verify_source "evidence source test"
) >"$workdir/toctou.log" 2>&1
then
    echo "evidence source test: post-capture mutation unexpectedly passed" >&2
    exit 1
fi
grep -F -q 'source or lockfile changed during evidence generation' "$workdir/toctou.log"

mkdir -p "$workdir/not-a-repo"
printf 'lock\n' >"$workdir/not-a-repo/Cargo.lock"
if (
    cd "$workdir/not-a-repo"
    . "$root/scripts/evidence-source.sh"
    evidence_capture_source "evidence source test"
) >"$workdir/missing-git.log" 2>&1
then
    echo "evidence source test: missing Git worktree unexpectedly passed" >&2
    exit 1
fi
grep -F -q 'evidence generation requires a Git worktree' "$workdir/missing-git.log"

echo "evidence source test: ok"
