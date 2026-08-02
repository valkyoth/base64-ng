#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-bytes/Cargo.toml"

fail() {
    echo "2.0 bytes: $1" >&2
    exit 1
}

if grep -R -E -n 'collect_buf|copy_to_bytes|to_vec\(' crates/base64-ng-bytes/src; then
    fail "full-input coalescing helper is present"
fi

for required_text in \
    'input.chunk()' \
    'input.advance(consumed)' \
    'output.put_slice(&scratch[..produced])' \
    'self.failed = true'
do
    if ! grep -F -q "$required_text" crates/base64-ng-bytes/src/driver.rs; then
        fail "stateful driver is missing required boundary: $required_text"
    fi
done

echo "2.0 bytes: no-default-features fragmented tests"
cargo test --manifest-path "$manifest" --no-default-features

echo "2.0 bytes: all-features fragmented, limit, rollback, and panic tests"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 bytes: release-mode contract tests"
cargo test --manifest-path "$manifest" --all-features --release

echo "2.0 bytes: lint and documentation"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 bytes: fragmented direct transforms, bounded progress, and rollback ok"
