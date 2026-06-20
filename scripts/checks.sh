#!/usr/bin/env sh
set -eu

echo "checks: formatting"
cargo fmt --all --check

echo "checks: release metadata"
scripts/validate-release-metadata.sh

echo "checks: crate publish plan"
scripts/release_crates.py --check
python3 scripts/test-release-crates.py

echo "checks: MSRV policy"
scripts/validate-msrv-policy.sh

echo "checks: documentation versions"
scripts/validate-doc-versions.sh

echo "checks: public API audit"
scripts/validate-api-audit.sh

echo "checks: file line budget"
scripts/validate-file-line-budget.sh

echo "checks: minimal dependency graph"
scripts/validate-dependencies.sh

echo "checks: companion crates"
scripts/check_companion_crates.sh

echo "checks: reserved feature placeholders"
scripts/check_reserved_features.sh

echo "checks: unsafe boundary"
scripts/validate-unsafe-boundary.sh

echo "checks: wasm wipe policy"
scripts/check_wasm_wipe_policy.sh

echo "checks: SIMD admission policy"
scripts/validate-simd-admission.sh

echo "checks: SIMD encode admission draft"
scripts/validate-simd-encode-admission-draft.sh

echo "checks: SIMD feature bundles"
scripts/check_simd_feature_bundles.sh

echo "checks: panic policy"
scripts/validate-panic-policy.sh

echo "checks: constant-time policy"
scripts/validate-constant-time-policy.sh

echo "checks: dudect timing harness"
scripts/check_dudect.sh

echo "checks: fuzz harness"
scripts/check_fuzz.sh

echo "checks: performance harness"
scripts/check_perf.sh

echo "checks: clippy default"
cargo clippy --all-targets -- -D warnings

echo "checks: clippy all features"
cargo clippy --all-targets --all-features -- -D warnings

echo "checks: no_std library build"
cargo check --no-default-features --lib

echo "checks: no-alloc portability smoke"
scripts/check_no_alloc_smoke.sh

echo "checks: migration guide smoke"
scripts/check_migration_smoke.sh

echo "checks: tests default"
cargo test --all-targets

echo "checks: tests all features"
cargo test --all-targets --all-features

echo "checks: tests no default features"
cargo test --no-default-features --all-targets

echo "checks: doctests"
cargo test --doc --all-features

echo "checks: doctests no default features"
cargo test --doc --no-default-features

echo "checks: docs"
cargo doc --no-deps --all-features

echo "checks: docs no default features"
cargo doc --no-deps --no-default-features

echo "checks: dependency policy"
cargo deny check

echo "checks: RustSec advisories"
cargo audit

echo "checks: license inventory"
cargo license --json >/tmp/base64-ng-cargo-license.json
test -s /tmp/base64-ng-cargo-license.json

echo "checks: ok"
