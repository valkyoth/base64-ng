#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-serde/Cargo.toml"
root="$(pwd)"
compile_fail_dir="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-serde-limit.XXXXXX")"
trap 'rm -rf "$compile_fail_dir"' EXIT HUP INT TERM

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
    'deserialize_with_limit' \
    'DEFAULT_SERDE_DECODE_MAX_LEN' \
    'MAX_SERDE_STACK_DECODED_BYTES' \
    'enforce_codec_input_limit' \
    'enforce_body_input_limit' \
    'decode_to_vec_with_limit' \
    'SecretArrayFrame::<CAP>::new' \
    'serialize_secret_codec' \
    'SecretInputVisitor' \
    'WipingOwnedInput::new' \
    'clear_bytes(self.bytes)' \
    'invalid secret base64 input' \
    'serializer.is_human_readable()'
do
    if ! grep -R -F -q "$required" crates/base64-ng-serde/src; then
        echo "2.0 serde: missing required implementation marker: $required" >&2
        exit 1
    fi
done

mkdir -p "$compile_fail_dir/src"
cat >"$compile_fail_dir/Cargo.toml" <<EOF
[package]
name = "base64-ng-serde-limit-contract"
version = "0.0.0"
edition = "2024"

[dependencies]
base64-ng-serde = { path = "$root/crates/base64-ng-serde" }
serde = { version = "1.0.229", default-features = false }

[workspace]
EOF
cat >"$compile_fail_dir/src/main.rs" <<'RS'
use serde::de::value::{BorrowedBytesDeserializer, Error};

fn main() {
    let input = b"AAAA";
    let deserializer = BorrowedBytesDeserializer::<Error>::new(input);
    let _decoded = base64_ng_serde::bounded::standard::deserialize::<_, 4097>(deserializer)
        .expect("oversized stack capacity must not compile");
}
RS
if cargo build --quiet --offline --manifest-path "$compile_fail_dir/Cargo.toml" \
    >"$compile_fail_dir/build.log" 2>&1; then
    echo "2.0 serde: oversized stack capacity compiled" >&2
    exit 1
fi
if ! grep -F -q 'Serde decoded capacity exceeds the supported 4096-byte stack limit' \
    "$compile_fail_dir/build.log"; then
    echo "2.0 serde: oversized stack capacity failed for an unexpected reason" >&2
    cat "$compile_fail_dir/build.log" >&2
    exit 1
fi

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

echo "2.0 serde: resource limits and pre-validation rejection"
cargo test --manifest-path "$manifest" --features alloc --test resource_limits

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
