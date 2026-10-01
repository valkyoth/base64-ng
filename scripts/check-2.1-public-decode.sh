#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    for features in '' alloc std simd std,simd checked-backend std,checked-backend; do
        cargo +"$compiler" test --locked --no-default-features --features "$features" --lib ordinary_decode::vector
        cargo +"$compiler" test --locked --no-default-features --features "$features" --lib public_validation_guard
        cargo +"$compiler" test --locked --no-default-features --features "$features" --test decode_validation
        cargo +"$compiler" test --locked --no-default-features --features "$features" --test simd_decode_dispatch
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
    cargo +"$compiler" test --locked --all-features --test v2_formatting_alloc strict_decode
done
sh scripts/check-2.1-baseline.sh
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 public decode: routing, faults, diagnostics, no-allocation and MSRV ok"
