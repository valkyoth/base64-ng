#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_INCREMENTAL_DECODER_TOOLCHAIN:-}"

test -s docs/2.0_INCREMENTAL_PADDED_DECODER.md
for required in \
    'incomplete encoded symbols with their original absolute indexes' \
    'current call leaves that call' \
    'byte always makes progress and never re-consumes' \
    'including space, tab, carriage return, line feed' \
    '`TruncatedInput` at the absolute end position' \
    'RFC4648-3.3-MUST-REJECT-NON-ALPHABET' \
    'every input/output partition' \
    'Kani proves bounded symbolic progress'
do
    if ! grep -F -q "$required" docs/2.0_INCREMENTAL_PADDED_DECODER.md; then
        echo "2.0 padded decoder: documentation is missing: $required" >&2
        exit 1
    fi
done

run_cargo() {
    if [ -n "$toolchain" ]; then
        cargo +"$toolchain" "$@"
    else
        cargo "$@"
    fi
}

for required in \
    'quantum: [u8; INPUT_QUANTUM]' \
    'quantum_indexes: [usize; INPUT_QUANTUM]' \
    'pending: [u8; OUTPUT_QUANTUM]' \
    'validate_partial_symbol' \
    'InputError::TrailingData' \
    'InputError::NonCanonicalTrailingBits' \
    'InputError::TruncatedInput' \
    'pub(crate) fn reset'
do
    if ! grep -F -q "$required" src/v2/incremental_decoder.rs; then
        echo "2.0 padded decoder: implementation is missing: $required" >&2
        exit 1
    fi
done

if rg -n -F \
    -e unsafe \
    -e 'std::' \
    -e 'alloc::' \
    -e 'Vec<' \
    -e 'Box<' \
    -e 'impl Drop' \
    -e '.unwrap(' \
    -e '.expect(' \
    -e 'panic!' \
    src/v2/incremental_decoder.rs
then
    echo "2.0 padded decoder: core gained allocation, panic, unsafe, or Drop" >&2
    exit 1
fi

harness=incremental_padded_decoder_progress_and_retry_are_bounded
if ! grep -F -q "fn $harness" src/kani_proofs.rs; then
    echo "2.0 padded decoder: missing Kani harness $harness" >&2
    exit 1
fi

run_cargo test --lib 'v2::incremental_decoder_tests'
run_cargo test --release --lib 'v2::incremental_decoder_tests'
run_cargo test --no-default-features --lib 'v2::incremental_decoder_tests'
run_cargo clippy --lib --all-features -- -D warnings

echo "2.0 padded decoder: strict chunking, errors, retry, and bounds evidence ok"
