#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    cargo +"$compiler" check --locked --no-default-features
    for features in alloc std alloc,simd std,simd alloc,checked-backend std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib forwarding_
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib append_tests
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib ordinary_string_tests
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
    cargo +"$compiler" test --locked --all-features --test v2_formatting_alloc
    cargo +"$compiler" test --locked --all-features --test decode_validation
done
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 forwarding: proof reuse, allocation, rollback, policy and MSRV passed"
