#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    cargo +"$compiler" test --locked --no-default-features --test decode_report
    cargo +"$compiler" clippy --locked --no-default-features --lib --tests -- -D warnings
    for features in alloc std alloc,simd std,simd alloc,checked-backend std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib composition_
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --test decode_report
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib backend_health::tests
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
done
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 decode composition: reports, health, static policies and checked recovery passed"
