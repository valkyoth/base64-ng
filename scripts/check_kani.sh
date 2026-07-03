#!/usr/bin/env sh
set -eu

if [ ! -d kani ]; then
    echo "Kani checks: skipping; kani/ is not present"
    exit 0
fi

kani_toolchain="${BASE64_NG_KANI_TOOLCHAIN:-1.90.0-x86_64-unknown-linux-gnu}"

if ! rustup toolchain list | grep -q "^$kani_toolchain"; then
    echo "Kani checks: skipping; Rust toolchain $kani_toolchain is not installed"
    exit 0
fi

cargo_kani() {
    rustup run "$kani_toolchain" cargo kani "$@"
}

if ! cargo_kani --version >/dev/null 2>&1; then
    echo "Kani checks: skipping; cargo kani is not installed"
    exit 0
fi

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

echo "Kani checks: using Rust toolchain $kani_toolchain"

if cargo_kani --no-default-features >"$log" 2>&1; then
    cat "$log"
    exit 0
else
    status="$?"
fi

if grep -q "Kani Rust Verifier" "$log" && grep -q "requires rustc" "$log"; then
    echo "Kani checks: skipping; installed Kani compiler is older than this crate's rust-version"
    exit 0
fi

cat "$log"
exit "$status"
