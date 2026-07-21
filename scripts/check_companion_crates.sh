#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-sanitization/Cargo.toml"
derive_manifest="crates/base64-ng-derive/Cargo.toml"
serde_manifest="crates/base64-ng-serde/Cargo.toml"
bytes_manifest="crates/base64-ng-bytes/Cargo.toml"
subtle_manifest="crates/base64-ng-subtle/Cargo.toml"
tokio_manifest="crates/base64-ng-tokio/Cargo.toml"

echo "companion crates: base64-ng-sanitization no-default-features"
cargo test --manifest-path "$manifest" --no-default-features

echo "companion crates: base64-ng-sanitization alloc"
cargo test --manifest-path "$manifest" --features alloc

echo "companion crates: base64-ng-sanitization std"
cargo test --manifest-path "$manifest" --features std

echo "companion crates: base64-ng-sanitization all features"
cargo test --manifest-path "$manifest" --all-features

echo "companion crates: base64-ng-sanitization clippy"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings

echo "companion crates: base64-ng-sanitization docs"
cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "companion crates: base64-ng-sanitization dependency policy"
scripts/cargo-deny-check.sh "$manifest" deny.toml

echo "companion crates: base64-ng-derive test"
cargo test --manifest-path "$derive_manifest"

echo "companion crates: base64-ng-derive clippy"
cargo clippy --manifest-path "$derive_manifest" --all-targets --all-features -- -D warnings

echo "companion crates: base64-ng-derive docs"
cargo doc --manifest-path "$derive_manifest" --no-deps --all-features

echo "companion crates: base64-ng-derive dependency policy"
scripts/cargo-deny-check.sh "$derive_manifest" deny.toml

echo "companion crates: base64-ng-serde test"
cargo test --manifest-path "$serde_manifest" --all-features

echo "companion crates: base64-ng-serde clippy"
cargo clippy --manifest-path "$serde_manifest" --all-targets --all-features -- -D warnings

echo "companion crates: base64-ng-serde docs"
cargo doc --manifest-path "$serde_manifest" --no-deps --all-features

echo "companion crates: base64-ng-serde dependency policy"
scripts/cargo-deny-check.sh "$serde_manifest" deny.toml

echo "companion crates: base64-ng-bytes test"
cargo test --manifest-path "$bytes_manifest" --all-features

echo "companion crates: base64-ng-bytes clippy"
cargo clippy --manifest-path "$bytes_manifest" --all-targets --all-features -- -D warnings

echo "companion crates: base64-ng-bytes docs"
cargo doc --manifest-path "$bytes_manifest" --no-deps --all-features

echo "companion crates: base64-ng-bytes dependency policy"
scripts/cargo-deny-check.sh "$bytes_manifest" deny.toml

echo "companion crates: base64-ng-subtle no-default-features"
cargo test --manifest-path "$subtle_manifest" --no-default-features

echo "companion crates: base64-ng-subtle all-features"
cargo test --manifest-path "$subtle_manifest" --all-features

echo "companion crates: base64-ng-subtle clippy"
cargo clippy --manifest-path "$subtle_manifest" --all-targets --all-features -- -D warnings

echo "companion crates: base64-ng-subtle docs"
cargo doc --manifest-path "$subtle_manifest" --no-deps --all-features

echo "companion crates: base64-ng-subtle dependency policy"
scripts/cargo-deny-check.sh "$subtle_manifest" deny.toml

echo "companion crates: base64-ng-tokio test"
if grep -R -n '\.fill(0)' crates/base64-ng-tokio/src; then
    echo "companion crates: base64-ng-tokio must use hardened wipe helpers" >&2
    exit 1
fi
cargo test --manifest-path "$tokio_manifest" --all-features

echo "companion crates: base64-ng-tokio clippy"
cargo clippy --manifest-path "$tokio_manifest" --all-targets --all-features -- -D warnings

echo "companion crates: base64-ng-tokio docs"
cargo doc --manifest-path "$tokio_manifest" --no-deps --all-features

echo "companion crates: base64-ng-tokio dependency policy"
scripts/cargo-deny-check.sh "$tokio_manifest" deny.toml

echo "companion crates: ok"
