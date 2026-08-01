#!/usr/bin/env sh
set -eu

document="docs/2.0_BACKEND_HEALTH.md"
for required in \
    'NeverRun' \
    'Testing' \
    'Healthy' \
    'Quarantined' \
    'initialize_backends()' \
    'StaticBackendToken::for_compiled_target()' \
    'StaticBackendToken::assume_supported' \
    'pointer-width atomics' \
    'Malformed caller input never changes' \
    'Secret operations never'
do
    if ! grep -F -q "$required" "$document"; then
        echo "2.0 backend health: documentation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'NEVER_RUN, TESTING' \
    'self.state.store(HEALTHY' \
    'self.state.store(QUARANTINED' \
    'run_catching_panics' \
    'bump_generation' \
    'pub fn initialize_backends()' \
    'pub(crate) fn quarantine('
do
    if ! grep -F -q "$required" src/v2/backend_health.rs; then
        echo "2.0 backend health: state machine is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'failed_self_test_is_permanently_quarantined' \
    'reentry_falls_back_and_initializer_panics_are_contained' \
    'comparison_faults_are_classified_without_trusting_backend_lengths'
do
    if ! grep -R -F -q "$required" src/v2/backend_health src/encode_backend src/decode_backend; then
        echo "2.0 backend health: fault injection is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'direct backend known-answer tests' \
    'direct_encode' \
    'direct_decode' \
    'STANDARD_ENCODED' \
    'URL_SAFE_ENCODED' \
    'BOUNDARY_INPUT' \
    'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
do
    if ! grep -F -qi "$required" src/v2/backend_health/kat.rs; then
        echo "2.0 backend health: direct KAT is missing: $required" >&2
        exit 1
    fi
done

echo "2.0 backend health: forced KAT and state tests"
cargo test --all-features --lib 'v2::backend_health'

echo "2.0 backend health: checked ordinary operation tests"
cargo test --features checked-backend --lib 'encode_backend'
cargo test --features checked-backend --lib 'decode_backend'
cargo test --all-features --lib 'simd::x86_encode_tests'

echo "2.0 backend health: scalar and no_std static fallbacks"
cargo check --no-default-features --lib
cargo check --no-default-features --features simd --lib

case "$(rustc -vV | sed -n 's/^host: //p')" in
    x86_64-*|i686-*)
        echo "2.0 backend health: compile-time AVX2 static selection"
        RUSTFLAGS='-C target-feature=+avx2' \
            cargo check --no-default-features --features simd --lib
        ;;
esac

scripts/validate-panic-policy.sh
scripts/validate-unsafe-boundary.sh

echo "2.0 backend health: KAT, quarantine, checked mode, and static selection ok"
