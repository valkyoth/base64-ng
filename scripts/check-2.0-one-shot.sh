#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_ONE_SHOT_TOOLCHAIN:-}"

run_cargo() {
    if [ -n "$toolchain" ]; then
        cargo +"$toolchain" "$@"
    else
        cargo "$@"
    fi
}

test -s docs/2.0_TRANSACTIONAL_ONE_SHOT.md
for required in \
    '`encode_into`' \
    '`decode_into`' \
    '`try_reserve_exact`' \
    'Every returned error' \
    'process-aborting allocator' \
    'No allocating secret operation'
do
    if ! grep -F -q "$required" docs/2.0_TRANSACTIONAL_ONE_SHOT.md; then
        echo "2.0 one-shot: documentation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub fn encode_into' \
    'pub fn decode_into' \
    'pub fn validate' \
    'pub fn encoded_len' \
    'pub fn decoded_len'
do
    if ! grep -F -q "$required" src/v2/ordinary.rs; then
        echo "2.0 one-shot: implementation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub fn encode_to_string' \
    'pub fn decode_to_vec' \
    'try_reserve_exact'
do
    if ! grep -F -q "$required" src/v2/ordinary_alloc.rs; then
        echo "2.0 one-shot: allocating implementation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub struct Base64String' \
    'pub fn encode' \
    'pub fn from_string' \
    'pub fn parse' \
    'pub fn decode_with_limit'
do
    if ! grep -F -q "$required" src/v2/ordinary_string.rs; then
        echo "2.0 one-shot: ordinary string implementation is missing: $required" >&2
        exit 1
    fi
done

run_cargo test --lib 'v2::one_shot_tests'
run_cargo test --no-default-features --lib 'v2::one_shot_tests'
scripts/check-2.0-migration-smoke.sh
run_cargo test --offline --manifest-path target/2_0_migration_smoke/Cargo.toml \
    transactional_2_0_surface_is_public_and_external
run_cargo clippy --lib --tests --all-features -- -D warnings

echo "2.0 one-shot: transactional slices, allocations, and public boundary ok"
