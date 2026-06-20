#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-sanitization/Cargo.toml"

echo "companion crates: base64-ng-sanitization no-default-features"
cargo test --manifest-path "$manifest" --no-default-features

echo "companion crates: base64-ng-sanitization alloc"
cargo test --manifest-path "$manifest" --features alloc

echo "companion crates: base64-ng-sanitization std"
cargo test --manifest-path "$manifest" --features std

echo "companion crates: base64-ng-sanitization clippy"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings

echo "companion crates: base64-ng-sanitization docs"
cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "companion crates: base64-ng-sanitization dependency policy"
cargo deny --manifest-path "$manifest" check --config deny.toml

echo "companion crates: ok"
