#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    for features in '' alloc std alloc,simd std,simd std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib in_place
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --test in_place_bulk
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
    cargo +"$compiler" test --locked --manifest-path fuzz/Cargo.toml --lib in_place
    cargo +"$compiler" clippy --locked --manifest-path fuzz/Cargo.toml --lib --bin in_place -- -D warnings
done
cargo test --locked --release --all-features --test v2_formatting_alloc in_place_bulk
sh scripts/validate-file-line-budget.sh
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 in-place bulk: overlap, recovery, transactionality, allocation and MSRV passed"
