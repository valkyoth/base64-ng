#!/usr/bin/env sh

# Shared source-provenance boundary for generated release evidence.

evidence_checksum_file() {
    evidence_file="$1"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$evidence_file"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$evidence_file"
    else
        cksum "$evidence_file"
    fi
}

evidence_capture_source() {
    evidence_label="$1"

    if ! EVIDENCE_SOURCE_COMMIT="$(git rev-parse --verify HEAD 2>/dev/null)"; then
        echo "$evidence_label: evidence generation requires a Git worktree" >&2
        exit 1
    fi
    if ! EVIDENCE_SOURCE_STATUS="$(git status --porcelain --untracked-files=all 2>/dev/null)"; then
        echo "$evidence_label: could not inspect worktree state" >&2
        exit 1
    fi
    if ! EVIDENCE_LOCK_RECORD="$(evidence_checksum_file Cargo.lock)"; then
        echo "$evidence_label: could not checksum Cargo.lock" >&2
        exit 1
    fi

    if [ -n "$EVIDENCE_SOURCE_STATUS" ]; then
        if [ "${BASE64_NG_ALLOW_DIRTY_EVIDENCE:-0}" != "1" ]; then
            echo "$evidence_label: refusing to generate release evidence from a dirty tree" >&2
            exit 1
        fi
        EVIDENCE_TREE_STATE="dirty-development-only"
    else
        EVIDENCE_TREE_STATE="clean"
    fi
}

evidence_verify_source() {
    evidence_label="$1"

    if ! evidence_current_commit="$(git rev-parse --verify HEAD 2>/dev/null)" ||
        ! evidence_current_status="$(git status --porcelain --untracked-files=all 2>/dev/null)" ||
        ! evidence_current_lock="$(evidence_checksum_file Cargo.lock)"
    then
        echo "$evidence_label: could not re-inspect source provenance" >&2
        exit 1
    fi

    if [ "$evidence_current_commit" != "$EVIDENCE_SOURCE_COMMIT" ] ||
        [ "$evidence_current_status" != "$EVIDENCE_SOURCE_STATUS" ] ||
        [ "$evidence_current_lock" != "$EVIDENCE_LOCK_RECORD" ]
    then
        echo "$evidence_label: source or lockfile changed during evidence generation" >&2
        exit 1
    fi
}

evidence_write_source_manifest() {
    echo "source:"
    echo "commit=$EVIDENCE_SOURCE_COMMIT"
    echo "tree_state=$EVIDENCE_TREE_STATE"
    printf '%s\n' "$EVIDENCE_LOCK_RECORD"
}

evidence_require_exact_manifest_key() {
    evidence_manifest_file="$1"
    evidence_manifest_key="$2"
    evidence_manifest_expected="$3"
    evidence_manifest_label="$4"

    if ! awk -v key="$evidence_manifest_key" -v expected="$evidence_manifest_expected" '
            index($0, key "=") == 1 && substr($0, 1, length(key) + 1) == key "=" {
                count += 1
                if (substr($0, length(key) + 2) != expected) {
                    invalid = 1
                }
            }
            END {
                exit !(count == 1 && invalid == 0)
            }
        ' "$evidence_manifest_file"
    then
        echo "$evidence_manifest_label: invalid or duplicate $evidence_manifest_key in $evidence_manifest_file" >&2
        exit 1
    fi
}

evidence_require_singleton_manifest_line() {
    evidence_manifest_file="$1"
    evidence_manifest_expected="$2"
    evidence_manifest_description="$3"
    evidence_manifest_label="$4"

    evidence_manifest_count="$(
        awk -v expected="$evidence_manifest_expected" '
            $0 == expected { count += 1 }
            END { print count + 0 }
        ' "$evidence_manifest_file"
    )"

    if [ "$evidence_manifest_count" -ne 1 ]; then
        echo "$evidence_manifest_label: missing or duplicate $evidence_manifest_description in $evidence_manifest_file" >&2
        exit 1
    fi
}

evidence_require_clean_source_manifest() {
    evidence_manifest_file="$1"
    evidence_manifest_commit="$2"
    evidence_manifest_label="$3"

    evidence_require_singleton_manifest_line \
        "$evidence_manifest_file" 'source:' 'source section' "$evidence_manifest_label"
    evidence_require_exact_manifest_key \
        "$evidence_manifest_file" commit "$evidence_manifest_commit" "$evidence_manifest_label"
    evidence_require_exact_manifest_key \
        "$evidence_manifest_file" tree_state clean "$evidence_manifest_label"
}
