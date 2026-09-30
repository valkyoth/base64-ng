#!/usr/bin/env sh
set -eu

for compiler in "$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)" 1.90.0; do
    for features in '' alloc std simd std,simd checked-backend std,checked-backend; do
        cargo +"$compiler" test --no-default-features --features "$features" --test decode_validation
        cargo +"$compiler" test --no-default-features --features "$features" --lib decode_validation::tests
        cargo +"$compiler" clippy --no-default-features --features "$features" --lib --test decode_validation -- -D warnings
    done
done
cargo test --doc --all-features decode_validation
cargo test --doc --all-features decode_into_with_validation
RUSTFLAGS="--cfg base64_ng_perf_evidence" cargo test --locked --manifest-path perf/public-api/Cargo.toml --features validation-policy,simd,checked
echo "2.1 validation policy: compatibility, reference execution, feature composition and MSRV ok"
