#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-tokio/Cargo.toml"
encoder="crates/base64-ng-tokio/src/encoder_writer.rs"
decoder="crates/base64-ng-tokio/src/decoder_writer.rs"
queue="crates/base64-ng-tokio/src/queue.rs"

fail() {
    echo "2.0 Tokio writers: $1" >&2
    exit 1
}

for source in "$encoder" "$decoder"
do
    for forbidden in \
        'Engine<' \
        'Alphabet' \
        '.encode_slice(' \
        '.decode_slice('
    do
        if grep -F -q "$forbidden" "$source"; then
            fail "independent legacy codec boundary remains in $source: $forbidden"
        fi
    done
done

if ! grep -F -q 'impl<const CAP: usize> Drop for OutputQueue<CAP>' "$queue"; then
    fail "fixed output queue is missing drop cleanup"
fi

for required in \
    'state: EncoderState' \
    'codec.encoder()' \
    'self.state.update(' \
    'self.state.finish(' \
    'self.state.clear()' \
    'self.input_accepted = self.state.source_position()' \
    'self.output_committed.checked_add(written)' \
    'self.output.discard_front(written)' \
    'catch_unwind(AssertUnwindSafe' \
    'resume_unwind(payload)' \
    'self.latch_failure()'
do
    if ! grep -F -q "$required" "$encoder"; then
        fail "encoder shared-state or commitment boundary is missing: $required"
    fi
done

for required in \
    'state: DecoderState' \
    'codec.decoder()' \
    'self.state.update(' \
    'self.state.finish(' \
    'self.state.clear()' \
    'self.input_accepted = self.state.source_position()' \
    'self.output_committed.checked_add(written)' \
    'self.output.discard_front(written)' \
    'catch_unwind(AssertUnwindSafe' \
    'resume_unwind(payload)' \
    'self.latch_failure()'
do
    if ! grep -F -q "$required" "$decoder"; then
        fail "decoder shared-state or commitment boundary is missing: $required"
    fi
done

echo "2.0 Tokio writers: compatibility and cleanup tests"
cargo test --manifest-path "$manifest" --all-features --lib
cargo test --manifest-path "$manifest" --all-features --test tokio_writer

echo "2.0 Tokio writers: adversarial schedules"
cargo test --manifest-path "$manifest" --all-features --test tokio_writer_adversarial

echo "2.0 Tokio writers: release-mode state-machine tests"
cargo test --manifest-path "$manifest" --all-features --release --test tokio_writer_adversarial

echo "2.0 Tokio writers: lint and documentation"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 Tokio writers: shared state, backpressure, cancellation, and cleanup ok"
