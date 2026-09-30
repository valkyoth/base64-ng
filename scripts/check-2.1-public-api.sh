#!/usr/bin/env sh
set -eu

python3 scripts/test-public-api-baseline.py
python3 scripts/test-public-api-sandbox.py
manifest=perf/public-api/Cargo.toml
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg base64_ng_perf_evidence"
cargo fmt --manifest-path "$manifest" --check
for features in '' alloc std std,simd std,simd,checked adapters,simd; do
    cargo test --locked --manifest-path "$manifest" --no-default-features --features "$features"
    cargo clippy --locked --manifest-path "$manifest" --all-targets --no-default-features --features "$features" -- -D warnings
done
cargo audit --file perf/public-api/Cargo.lock
scripts/cargo-deny-check.sh "$manifest" perf/deny.toml
echo "2.1 public API baseline: oracle, operation, measurement, and feature checks passed"
