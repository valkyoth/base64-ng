#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_INCREMENTAL_DECODER_TOOLCHAIN:-}"

test -s docs/2.0_INCREMENTAL_DECODER_FINALIZATION.md
for required in \
    'forbid every `=` byte' \
    'two-symbol tail produces one byte' \
    'three-symbol tail produces two bytes' \
    'one-symbol unpadded tail is an impossible' \
    'retry with one output byte produces exactly one byte' \
    '`InputAfterFinish`' \
    'All 64 one-symbol, 4,096 two-symbol, and 262,144 three-symbol' \
    'every input and output partition'
do
    if ! grep -F -q "$required" docs/2.0_INCREMENTAL_DECODER_FINALIZATION.md; then
        echo "2.0 decoder finalization: documentation is missing: $required" >&2
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
    'impl<S: Codec> Base64<S>' \
    'DecodePadding::Forbid' \
    'decode_unpadded_tail' \
    'InputError::InvalidLength' \
    'InputError::NonCanonicalTrailingBits' \
    'lifecycle.begin_finish'
do
    if ! grep -F -q "$required" src/v2/incremental_decoder.rs; then
        echo "2.0 decoder finalization: implementation is missing: $required" >&2
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
    src/v2/incremental_decoder.rs \
    src/v2/lifecycle.rs
then
    echo "2.0 decoder finalization: core gained allocation, panic, unsafe, or Drop" >&2
    exit 1
fi

harness=incremental_unpadded_decoder_finish_and_retry_are_bounded
if ! grep -F -q "fn $harness" src/kani_proofs.rs; then
    echo "2.0 decoder finalization: missing Kani harness $harness" >&2
    exit 1
fi

run_cargo test --lib 'v2::incremental_decoder_unpadded_tests'
run_cargo test --release --lib 'v2::incremental_decoder_unpadded_tests'
run_cargo test --no-default-features --lib 'v2::incremental_decoder_unpadded_tests'
run_cargo clippy --lib --all-features -- -D warnings

echo "2.0 decoder finalization: strict padded/unpadded tail evidence ok"
