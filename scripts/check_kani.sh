#!/usr/bin/env sh
set -eu

if [ ! -d kani ]; then
    echo "Kani checks: skipping; kani/ is not present"
    exit 0
fi

if ! cargo kani --version >/dev/null 2>&1; then
    echo "Kani checks: skipping; cargo kani is not installed"
    exit 0
fi

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

if cargo kani --no-default-features >"$log" 2>&1; then
    cat "$log"
    exit 0
fi

status="$?"

if grep -q "Kani Rust Verifier" "$log" && grep -q "requires rustc" "$log"; then
    echo "Kani checks: skipping; installed Kani compiler is older than this crate's rust-version"
    exit 0
fi

cat "$log"
exit "$status"
