#!/usr/bin/env sh
set -eu

python3 scripts/test-ssse3-validation-asm.py

active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
host="$(rustc -vV | sed -n 's/^host: //p')"
case "$host" in
    x86_64-*|i686-*|i586-*) ;;
    *) echo "2.1 SSSE3 validation: skipping non-x86 host $host"; exit 0 ;;
esac

for compiler in "$active" 1.90.0; do
    for features in std std,simd std,simd,checked-backend; do
        cargo +"$compiler" test --locked --no-default-features --features "$features" \
            --lib ssse3_candidate -- --nocapture
        cargo +"$compiler" test --locked --no-default-features --features "$features" \
            --lib validation_guard
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" \
            --lib --tests -- -D warnings
    done
    cargo +"$compiler" check --locked --no-default-features
    cargo +"$compiler" check --locked --no-default-features --features simd
done

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM
CARGO_TARGET_DIR="$tmp" RUSTFLAGS='-C target-feature=+ssse3,+sse4.1' \
    cargo rustc --locked --release --all-features --lib -- --emit=asm --test
python3 scripts/check-ssse3-validation-asm.py "$tmp/release/deps"
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 SSSE3 validation: candidate semantics, bounds, MSRV and assembly ok; public dispatch unchanged"
