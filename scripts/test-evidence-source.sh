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

manifest="$workdir/source-manifest.txt"
commit="0123456789abcdef0123456789abcdef01234567"

assert_manifest_accepted() {
    if ! (
        . "$root/scripts/evidence-source.sh"
        evidence_require_clean_source_manifest \
            "$manifest" "$commit" "evidence manifest test"
    ) >"$workdir/manifest-accepted.log" 2>&1
    then
        echo "evidence source test: canonical source manifest was rejected" >&2
        cat "$workdir/manifest-accepted.log" >&2
        exit 1
    fi
}

assert_manifest_rejected() {
    mutation="$1"
    if (
        . "$root/scripts/evidence-source.sh"
        evidence_require_clean_source_manifest \
            "$manifest" "$commit" "evidence manifest test"
    ) >"$workdir/manifest-$mutation.log" 2>&1
    then
        echo "evidence source test: $mutation source manifest unexpectedly passed" >&2
        exit 1
    fi
    grep -E -q 'invalid or duplicate|missing or duplicate' \
        "$workdir/manifest-$mutation.log"
}

printf 'source:\ncommit=%s\ntree_state=clean\n' "$commit" >"$manifest"
assert_manifest_accepted

printf 'source:\ncommit=%s\ncommit=%s\ntree_state=clean\n' \
    "$commit" "$commit" >"$manifest"
assert_manifest_rejected duplicate-identical-key

printf 'source:\ncommit=stale\ncommit=%s\ntree_state=clean\n' \
    "$commit" >"$manifest"
assert_manifest_rejected conflicting-key

printf 'source:\ncommit=%s\ntree_state=clean\nsource:\ncommit=%s\ntree_state=clean\n' \
    "$commit" "$commit" >"$manifest"
assert_manifest_rejected duplicate-source-section

printf 'source:\nsource:\ncommit=%s\ntree_state=clean\n' "$commit" >"$manifest"
assert_manifest_rejected duplicate-source-header

printf 'source:\nprefix_commit=%s\ntree_state=clean\n' "$commit" >"$manifest"
assert_manifest_rejected prefixed-key

printf 'source:\ncommit_suffix=%s\ntree_state=clean\n' "$commit" >"$manifest"
assert_manifest_rejected suffixed-key

printf 'source:\ncommit=%s-suffix\ntree_state=clean\n' "$commit" >"$manifest"
assert_manifest_rejected suffixed-value

printf 'source:\ncommit=%s\ntree_state=clean-but-untrusted\n' "$commit" >"$manifest"
assert_manifest_rejected suffixed-tree-state

printf 'source:\ncommit=stale\ntree_state=dirty-development-only\ncommit=%s\ntree_state=clean\n' \
    "$commit" >"$manifest"
assert_manifest_rejected stale-followed-by-current

echo "evidence source test: ok"
