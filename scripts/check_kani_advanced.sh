#!/usr/bin/env sh
set -eu

if [ ! -d kani ]; then
    echo "Advanced Kani checks: skipping; kani/ is not present"
    exit 0
fi

if ! cargo kani --version >/dev/null 2>&1; then
    echo "Advanced Kani checks: skipping; cargo kani is not installed"
    exit 0
fi

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

if [ -n "${RUSTFLAGS:-}" ]; then
    export RUSTFLAGS="$RUSTFLAGS --cfg base64_ng_kani_advanced"
else
    export RUSTFLAGS="--cfg base64_ng_kani_advanced"
fi

echo "Advanced Kani checks: running opt-in expensive harness set"
if cargo kani --no-default-features >"$log" 2>&1; then
    cat "$log"
    exit 0
else
    status="$?"
fi

if grep -q "Kani Rust Verifier" "$log" && grep -q "requires rustc" "$log"; then
    echo "Advanced Kani checks: skipping; installed Kani compiler is older than this crate's rust-version"
    exit 0
fi

cat "$log"
exit "$status"
