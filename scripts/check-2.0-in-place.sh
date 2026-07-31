#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_IN_PLACE_TOOLCHAIN:-}"

run_cargo() {
    if [ -n "$toolchain" ]; then
        cargo +"$toolchain" "$@"
    else
        cargo "$@"
    fi
}

test -s docs/2.0_IN_PLACE_OPERATIONS.md
for required in \
    '`Base64::encode_in_place' \
    '`Base64::decode_in_place' \
    '`Base64::decode_in_place_staged' \
    'byte-disjoint' \
    'fixed-work claim ends at the result gate' \
    'leaves both ranges byte-for-byte unchanged' \
    'shared cursor-length helpers' \
    'Commit 40'
do
    if ! grep -F -q "$required" docs/2.0_IN_PLACE_OPERATIONS.md; then
        echo "2.0 in-place: documentation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub enum InPlaceError' \
    'pub fn encode_in_place' \
    'pub fn decode_in_place' \
    'checked_add(left_len)' \
    'checked_add(right_len)' \
    'encoded_tail_len(' \
    'quantum_decoded_len(' \
    'tail_decoded_len(' \
    'fn encode_reverse' \
    'fn decode_forward'
do
    if ! grep -F -q "$required" src/v2/in_place.rs; then
        echo "2.0 in-place: ordinary implementation is missing: $required" >&2
        exit 1
    fi
done

for helper in \
    'encoded_tail_len' \
    'quantum_decoded_len' \
    'tail_decoded_len'
do
    runtime_occurrences=$(grep -F -c "$helper(" src/v2/in_place.rs)
    if [ "$runtime_occurrences" -lt 2 ] \
        || ! grep -F -q "$helper(" src/kani_in_place_proofs.rs \
        || ! grep -F -q "$helper(" src/v2/in_place_tests.rs
    then
        echo "2.0 in-place: runtime, tests, and Kani must share $helper" >&2
        exit 1
    fi
done

for required in \
    'pub fn secret_decode_staging_len' \
    'pub fn decode_in_place_staged' \
    'require_disjoint_slices' \
    'ct_error_gate_barrier' \
    'InvalidSecretInput' \
    'wipe_bytes(private_staging)' \
    'wipe_bytes(buffer)'
do
    if ! grep -F -q "$required" src/v2/secret_in_place.rs; then
        echo "2.0 in-place: secret implementation is missing: $required" >&2
        exit 1
    fi
done

for harness in \
    'reverse_in_place_encode_never_overwrites_unread_input' \
    'forward_in_place_decode_writes_only_consumed_prefixes'
do
    if ! grep -F -q "fn $harness" src/kani_in_place_proofs.rs; then
        echo "2.0 in-place: Kani harness is missing: $harness" >&2
        exit 1
    fi
done

run_cargo test --lib 'v2::in_place_tests'
run_cargo test --lib 'v2::secret_in_place_tests'
run_cargo test --no-default-features --lib 'v2::in_place_tests'
run_cargo test --no-default-features --lib 'v2::secret_in_place_tests'
scripts/check-2.0-migration-smoke.sh
run_cargo test --offline --manifest-path target/2_0_migration_smoke/Cargo.toml \
    in_place_2_0_surface_is_public_and_external
run_cargo clippy --lib --tests --all-features -- -D warnings

echo "2.0 in-place: reverse, forward, staged secret, overlap, and cleanup contracts ok"
