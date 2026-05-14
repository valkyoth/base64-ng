#!/usr/bin/env sh
set -eu

echo "backend evidence: runtime report"
cargo test --test rfc4648 --all-features runtime_backend_report_keeps_scalar_active -- --nocapture

echo "backend evidence: SIMD prototype scalar-equivalence tests"
cargo test --all-features simd::tests:: -- --nocapture
