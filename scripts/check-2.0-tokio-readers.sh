#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-tokio/Cargo.toml"
reader_source="crates/base64-ng-tokio/src/readers.rs"

fail() {
    echo "2.0 Tokio readers: $1" >&2
    exit 1
}

for forbidden in \
    'Engine<' \
    'Alphabet' \
    '.encode_slice(' \
    '.decode_slice('
do
    if grep -F -q "$forbidden" "$reader_source"; then
        fail "independent legacy codec boundary remains: $forbidden"
    fi
done

for required in \
    'state: EncoderState' \
    'state: DecoderState' \
    'codec.encoder()' \
    'codec.decoder()' \
    'Boundary::Exact' \
    'this.boundary.consume(read)' \
    'catch_unwind(AssertUnwindSafe' \
    'resume_unwind(payload)' \
    'self.state.clear()' \
    'self.failed = true'
do
    if ! grep -F -q "$required" "$reader_source"; then
        fail "shared-state or fail-closed boundary is missing: $required"
    fi
done

echo "2.0 Tokio readers: tests and adversarial schedules"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 Tokio readers: release-mode state-machine tests"
cargo test --manifest-path "$manifest" --all-features --release --test tokio_reader_adversarial

echo "2.0 Tokio readers: lint and documentation"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 Tokio readers: shared state, exact boundaries, cancellation, and cleanup ok"
