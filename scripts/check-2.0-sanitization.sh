#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-sanitization/Cargo.toml"
source_dir="crates/base64-ng-sanitization/src"

echo "2.0 sanitization: protected-fill policy"
for required in \
    'SanitizationProtectedDecodeExt' \
    'SecretFrame::new' \
    'LockedSecretBytes::<N>::zeroed_with_protection' \
    'LockedSecretVec::try_from_exact_len_with_protection' \
    'LockedSecretVec::try_from_capacity_bounded_with_protection' \
    'ProtectedAllocation::Staging' \
    'ProtectedAllocation::Destination'
do
    if ! grep -R -F -q "$required" "$source_dir"; then
        echo "2.0 sanitization: missing protected-fill marker: $required" >&2
        exit 1
    fi
done

if grep -F -n 'post-construction report admission' \
    crates/base64-ng-sanitization/src/lib.rs; then
    echo "2.0 sanitization: degraded compatibility default remains" >&2
    exit 1
fi

echo "2.0 sanitization: no-default-features"
RUSTFLAGS="-D warnings" cargo check --manifest-path "$manifest" --no-default-features

echo "2.0 sanitization: memory-lock without std"
RUSTFLAGS="-D warnings" cargo check --manifest-path "$manifest" --no-default-features --features memory-lock

echo "2.0 sanitization: protected tests"
cargo test --manifest-path "$manifest" --all-features protected_tests

echo "2.0 sanitization: complete suite"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 sanitization: lint"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings

echo "2.0 sanitization: documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 sanitization: protected staging, result gate, and destination ok"
