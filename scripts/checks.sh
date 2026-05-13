#!/usr/bin/env sh
set -eu

echo "checks: formatting"
cargo fmt --all --check

echo "checks: release metadata"
scripts/validate-release-metadata.sh

echo "checks: minimal dependency graph"
scripts/validate-dependencies.sh

echo "checks: unsafe boundary"
scripts/validate-unsafe-boundary.sh

echo "checks: clippy default"
cargo clippy --all-targets -- -D warnings

echo "checks: clippy all features"
cargo clippy --all-targets --all-features -- -D warnings

echo "checks: no_std library build"
cargo check --no-default-features --lib

echo "checks: tests default"
cargo test --all-targets

echo "checks: tests all features"
cargo test --all-targets --all-features

echo "checks: tests no default features"
cargo test --no-default-features --all-targets

echo "checks: doctests"
cargo test --doc --all-features

echo "checks: docs"
cargo doc --no-deps --all-features

echo "checks: dependency policy"
cargo deny check

echo "checks: RustSec advisories"
cargo audit

echo "checks: license inventory"
cargo license --json >/tmp/base64-ng-cargo-license.json
test -s /tmp/base64-ng-cargo-license.json

echo "checks: ok"
