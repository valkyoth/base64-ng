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
    'ProtectedAllocation::Destination' \
    'DEFAULT_SECRET_VEC_DECODE_MAX_LEN' \
    'enforce_encoded_input_limit' \
    'EncodedInputLimit' \
    'try_reserve_exact' \
    'enforce_stack_secret_capacity'
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

echo "2.0 sanitization: bounded allocation and stack policy"
cargo test --manifest-path "$manifest" --all-features \
    bounded_secret_vec_decode_enforces_public_capacity_before_output
cargo test --manifest-path "$manifest" --all-features \
    staged_secret_vec_decode_rejects_stage_before_output_allocation
cargo test --manifest-path "$manifest" --all-features \
    default_secret_vec_decode_rejects_output_above_one_mibibyte
cargo test --manifest-path "$manifest" --all-features \
    capacity_overflow_is_a_reported_allocation_failure
cargo test --manifest-path "$manifest" --all-features \
    capacity_limit_returns_without_calling_allocator
cargo test --manifest-path "$manifest" --all-features \
    staging_limit_returns_without_calling_allocator
cargo test --manifest-path "$manifest" --all-features \
    encoded_input_limit_returns_without_calling_allocator
cargo test --manifest-path "$manifest" --all-features \
    fixed_secret_bytes_reject_oversized_input_before_validation
cargo test --manifest-path "$manifest" --all-features \
    bounded_dynamic_decode_rejects_oversized_input_before_validation

echo "2.0 sanitization: lint"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings

echo "2.0 sanitization: documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 sanitization: protected staging, bounded allocation, stack limit, result gate, and destination ok"
