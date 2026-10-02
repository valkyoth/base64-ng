#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    for features in '' alloc std simd std,simd checked-backend std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib ordinary_encode
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib encode_backend::checked
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib append_tests
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib formatting_tests
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
    cargo +"$compiler" test --locked --all-features --test v2_formatting_alloc
done
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 public encode: routing, recovery, sinks, allocation and MSRV passed"
