#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-serde/Cargo.toml"

echo "2.0 serde: validated codec routing"
if grep -R -E -n '(Engine|Profile|STANDARD_NO_PAD|URL_SAFE_NO_PAD|decode_vec|String::deserialize)' \
    crates/base64-ng-serde/src; then
    echo "2.0 serde: legacy engine, profile, or encoded-input copy remains" >&2
    exit 1
fi

for required in \
    'STRICT_STANDARD_PADDED' \
    'STRICT_STANDARD_UNPADDED' \
    'STRICT_URL_SAFE_PADDED' \
    'STRICT_URL_SAFE_UNPADDED' \
    'MIME_BODY_STRICT' \
    'PEM_BODY_LF' \
    'deserialize_bounded' \
    'SecretArrayFrame::<CAP>::new' \
    'serialize_secret_codec' \
    'clear_bytes(&mut encoded)' \
    'invalid secret base64 input' \
    'serializer.is_human_readable()'
do
    if ! grep -R -F -q "$required" crates/base64-ng-serde/src; then
        echo "2.0 serde: missing required implementation marker: $required" >&2
        exit 1
    fi
done

if grep -E -n 'VecDecoder.*String|copy_payload_into' crates/base64-ng-serde/src/adapter.rs; then
    echo "2.0 serde: encoded input must not be materialized or compacted" >&2
    exit 1
fi

echo "2.0 serde: no-default-features"
RUSTFLAGS="-D warnings" cargo check --manifest-path "$manifest" --no-default-features

echo "2.0 serde: alloc-only feature boundary"
RUSTFLAGS="-D warnings" cargo check --manifest-path "$manifest" --features alloc

echo "2.0 serde: allocation compatibility"
cargo test --manifest-path "$manifest" --features alloc --test serde

echo "2.0 serde: bounded and secret adapters"
cargo test --manifest-path "$manifest" --all-features --test bounded

echo "2.0 serde: human-readable and binary modes"
cargo test --manifest-path "$manifest" --all-features --test serializer_modes

echo "2.0 serde: complete suite"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 serde: optimized suite"
cargo test --manifest-path "$manifest" --release --all-features

echo "2.0 serde: lint"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings

echo "2.0 serde: documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 serde: bounded representation and secret handling ok"
