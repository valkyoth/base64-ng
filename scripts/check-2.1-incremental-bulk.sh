#!/usr/bin/env sh
set -eu
python3 scripts/test-incremental-decoder-policy.py
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    for features in '' alloc std alloc,simd std,simd alloc,checked-backend std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib incremental
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --test incremental_bulk
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
done
cargo test --locked --release --all-features --test v2_formatting_alloc incremental_bulk
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 incremental bulk: progress, transactionality, recovery and MSRV passed"
