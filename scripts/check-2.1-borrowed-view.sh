#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    for features in '' alloc std alloc,simd std,simd alloc,checked-backend std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib borrowed_view
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --test decode_ref
        cargo +"$compiler" test --locked --no-default-features --features "$features" --doc ordinary_ref
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
    cargo +"$compiler" test --locked --release --all-features --lib composition_quarantine_between_validation_and_write_invalidates_tokens
    cargo +"$compiler" test --locked --release --all-features --test v2_formatting_alloc borrowed_view
done
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 borrowed view: lifetime, reuse, health, checked recovery and MSRV passed"
