#!/usr/bin/env sh
set -eu

if ! rustc --print cfg | grep -q 'target_endian="big"'; then
    echo "big-endian hardware checks: this gate must run natively on real big-endian hardware" >&2
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "big-endian hardware checks: worktree must be clean" >&2
    exit 1
fi

commit="$(git rev-parse HEAD)"
if ! printf '%s\n' "$commit" | grep -E -q '^[0-9a-f]{40}$'; then
    echo "big-endian hardware checks: HEAD is not a full Git object id" >&2
    exit 1
fi

evidence_dir="target/release-evidence/big-endian-hardware"
transcript="$evidence_dir/output.txt"
mkdir -p "$evidence_dir"
: >"$transcript"

run() {
    printf '%s\n' "+ $*" >>"$transcript"
    if "$@" >>"$transcript" 2>&1; then
        return
    fi
    cat "$transcript" >&2
    exit 1
}

run rustc -Vv
run cargo -V
run uname -a
run cargo test --all-targets
run cargo test --all-targets --all-features
run cargo test --no-default-features --all-targets
run cargo test --doc --all-features
run cargo test --doc --no-default-features
run scripts/validate-big-endian-byte-order.sh

cat "$transcript"
sha256="$(sha256sum "$transcript" | sed 's/ .*//')"
echo "big-endian hardware checks: source_commit=$commit"
echo "big-endian hardware checks: output_sha256=$sha256"
echo "big-endian hardware checks: ok"
