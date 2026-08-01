#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_INCREMENTAL_ENCODER_TOOLCHAIN:-}"

test -s docs/2.0_INCREMENTAL_ENCODER.md
for required in \
    'incomplete input bytes, at most four pending encoded bytes' \
    'no allocation' \
    'is one byte, and retrying does not re-consume input.' \
    'idempotent: later `finish` calls return `Complete`' \
    'There is no malformed-input or forgiving' \
    'RFC 4648 Section 10 vectors' \
    'every input/output partition' \
    'Kani proves bounded progress'
do
    if ! grep -F -q "$required" docs/2.0_INCREMENTAL_ENCODER.md; then
        echo "2.0 incremental encoder: documentation is missing: $required" >&2
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
    'tail: [u8; INPUT_QUANTUM]' \
    'pending: [u8; OUTPUT_QUANTUM]' \
    'planned_input_consumption' \
    'NonZeroUsize::MIN' \
    'encode_tail' \
    'pub fn reset'
do
    if ! grep -F -q "$required" src/v2/incremental.rs; then
        echo "2.0 incremental encoder: implementation is missing: $required" >&2
        exit 1
    fi
done

if grep -n -F \
    -e unsafe \
    -e 'std::' \
    -e 'alloc::' \
    -e 'Vec<' \
    -e 'Box<' \
    -e 'impl Drop' \
    -e '.unwrap(' \
    -e '.expect(' \
    -e 'panic!' \
    src/v2/incremental.rs
then
    echo "2.0 incremental encoder: core gained allocation, panic, unsafe, or Drop" >&2
    exit 1
fi

for harness in \
    incremental_standard_encoder_progress_and_state_are_bounded \
    incremental_standard_encoder_finish_is_bounded
do
    if ! grep -F -q "fn $harness" src/kani_proofs.rs; then
        echo "2.0 incremental encoder: missing Kani harness $harness" >&2
        exit 1
    fi
done

run_cargo test --lib 'v2::incremental_encoder_tests'
run_cargo test --release --lib 'v2::incremental_encoder_tests'
run_cargo test --no-default-features --lib 'v2::incremental_encoder_tests'
run_cargo clippy --lib --all-features -- -D warnings

echo "2.0 incremental encoder: chunking, retry, canonical-tail, and bounds evidence ok"
