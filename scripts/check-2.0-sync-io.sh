#!/usr/bin/env sh
set -eu

fail() {
    echo "2.0 synchronous I/O: $1" >&2
    exit 1
}

for source in \
    src/stream/encoder.rs \
    src/stream/decoder.rs \
    src/stream/encoder_reader.rs \
    src/stream/decoder_reader.rs
do
    if grep -Eq 'pending: \[u8; [234]\]' "$source"; then
        fail "$source owns an independent Base64 input quantum"
    fi
    if grep -Eq 'engine\.(encode|decode)_slice' "$source"; then
        fail "$source bypasses the shared incremental driver"
    fi
done

grep -Fq 'driver: EncoderDriver' src/stream/encoder.rs || \
    fail "encoder writer is not delegated"
grep -Fq 'driver: DecoderDriver' src/stream/decoder.rs || \
    fail "decoder writer is not delegated"
grep -Fq 'driver: EncoderDriver' src/stream/encoder_reader.rs || \
    fail "encoder reader is not delegated"
grep -Fq 'driver: DecoderDriver' src/stream/decoder_reader.rs || \
    fail "decoder reader is not delegated"

cargo test --all-features --test stream

echo "2.0 synchronous I/O: shared incremental drivers and I/O contracts ok"
