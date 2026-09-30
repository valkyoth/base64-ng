#!/usr/bin/env sh
set -eu

python3 scripts/test-x86-validation-asm.py
python3 scripts/test-x86-validation-ir.py
python3 scripts/test-2.0-skeleton.py
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
host="$(rustc -vV | sed -n 's/^host: //p')"
case "$host" in
    x86_64-*|i686-*|i586-*) ;;
    *) echo "2.1 AVX-512 validation: skipping non-x86 host $host"; exit 0 ;;
esac
for compiler in "$active" 1.90.0; do
    for features in std std,simd std,simd,checked-backend; do
        cargo +"$compiler" test --locked --no-default-features --features "$features" \
            --lib avx512_candidate -- --nocapture
        cargo +"$compiler" test --locked --no-default-features --features "$features" \
            --lib avx512_validation_guard
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" \
            --lib --tests -- -D warnings
    done
    cargo +"$compiler" check --locked --no-default-features
    cargo +"$compiler" check --locked --no-default-features --features simd
done

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM
CARGO_TARGET_DIR="$tmp/production" \
    cargo rustc --locked --release --all-features --lib -- --emit=llvm-ir
python3 scripts/check-x86-validation-ir.py "$tmp/production/release/deps"
CARGO_TARGET_DIR="$tmp/test" RUSTFLAGS='-C target-feature=+ssse3,+sse4.1' \
    cargo rustc --locked --release --all-features --lib -- --emit=asm --test
python3 scripts/check-x86-validation-asm.py "$tmp/test/release/deps" avx512
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 AVX-512 validation: candidate semantics, bounds, MSRV and assembly ok; public dispatch unchanged"
