#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_CONST_BUFFER_TOOLCHAIN:-}"
workdir="target/2_0_const_buffers"
case_dir="$workdir/cases"
source_dir="$workdir/src"
evidence_dir="target/release-evidence/2.0-const-buffers"
mkdir -p "$case_dir" "$source_dir" "$evidence_dir"

run_cargo() {
    if [ -n "$toolchain" ]; then
        cargo +"$toolchain" "$@"
    else
        cargo "$@"
    fi
}

run_rustc() {
    if [ -n "$toolchain" ]; then
        rustup run "$toolchain" rustc "$@"
    else
        rustc "$@"
    fi
}

test -s docs/2.0_CONST_AND_BOUNDED_BUFFERS.md
for required in \
    'exact output length' \
    '`ConstTransformError`' \
    '`EncodedArray<CAP>`' \
    '`SecretArray<CAP>` is non-Clone' \
    'footprints, not measurements of the complete dynamic call chain' \
    'best-effort wipe limitations'
do
    if ! grep -F -q "$required" docs/2.0_CONST_AND_BOUNDED_BUFFERS.md; then
        echo "2.0 const buffers: documentation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub const fn encode_array' \
    'pub const fn decode_array' \
    'pub const fn encoded_len' \
    'pub const fn decoded_len'
do
    if ! grep -F -q "$required" src/v2/const_transforms.rs; then
        echo "2.0 const buffers: const implementation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub struct EncodedArray' \
    'pub struct DecodedArray' \
    'pub struct SecretArray' \
    'impl<const CAP: usize> Drop for SecretArray' \
    'crate::wipe_tail(&mut bytes, len)' \
    'crate::wipe_bytes(&mut self.bytes)'
do
    if ! grep -F -q "$required" src/v2/bounded.rs; then
        echo "2.0 const buffers: bounded implementation is missing: $required" >&2
        exit 1
    fi
done

if grep -F -q 'Drop for EncodedArray' src/v2/bounded.rs \
    || grep -F -q 'Drop for DecodedArray' src/v2/bounded.rs
then
    echo "2.0 const buffers: ordinary arrays gained drop-time cleanup" >&2
    exit 1
fi

for harness in \
    bounded_array_visible_length_is_checked \
    const_standard_encode_is_exact_and_bounded
do
    if ! grep -F -q "fn $harness" src/kani_proofs.rs; then
        echo "2.0 const buffers: Kani harness is missing: $harness" >&2
        exit 1
    fi
done

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-const-buffer-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", default-features = false }
TOML

cat >"$case_dir/valid.rs" <<'RS'
use base64_ng::{
    CodecBuilder, DecodePadding, DecodedArray, EncodePadding, EncodedArray, SecretArray,
    STRICT_STANDARD_PADDED,
};

const TABLE: [u8; 64] =
    *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const CUSTOM: base64_ng::Base64<base64_ng::RuntimeSpec> =
    match CodecBuilder::from_table(TABLE) {
        Ok(builder) => match builder
            .encode_padding(EncodePadding::Unpadded)
            .decode_padding(DecodePadding::Forbid)
            .build()
        {
            Ok(codec) => codec,
            Err(_) => panic!("valid policy rejected"),
        },
        Err(_) => panic!("valid alphabet rejected"),
    };
const ENCODED: [u8; 8] = match STRICT_STANDARD_PADDED.encode_array(b"hello") {
    Ok(output) => output,
    Err(_) => panic!("valid literal rejected"),
};
const DECODED: [u8; 5] = match STRICT_STANDARD_PADDED.decode_array(&ENCODED) {
    Ok(output) => output,
    Err(_) => panic!("valid literal rejected"),
};
const CUSTOM_ENCODED: [u8; 8] = match CUSTOM.encode_array(b"custom") {
    Ok(output) => output,
    Err(_) => panic!("valid custom literal rejected"),
};

fn assert_copy<T: Copy>() {}
fn assert_send_sync<T: Send + Sync>() {}

fn main() {
    assert_eq!(ENCODED, *b"aGVsbG8=");
    assert_eq!(DECODED, *b"hello");
    assert_eq!(CUSTOM_ENCODED, *b"W1TxbE7r");
    assert_copy::<EncodedArray<64>>();
    assert_copy::<DecodedArray<64>>();
    assert_send_sync::<SecretArray<64>>();
}
RS

cat >"$case_dir/invalid-literal.rs" <<'RS'
use base64_ng::STRICT_STANDARD_PADDED;

const _: [u8; 3] = match STRICT_STANDARD_PADDED.decode_array(b"Zm!v") {
    Ok(output) => output,
    Err(_) => panic!("invalid const base64 literal"),
};

fn main() {}
RS

cat >"$case_dir/invalid-padding.rs" <<'RS'
use base64_ng::STRICT_STANDARD_PADDED;

const _: [u8; 1] = match STRICT_STANDARD_PADDED.decode_array(b"Zg=") {
    Ok(output) => output,
    Err(_) => panic!("invalid const base64 padding"),
};

fn main() {}
RS

cat >"$case_dir/invalid-alphabet.rs" <<'RS'
use base64_ng::{Base64, CodecBuilder, RuntimeSpec};

const _: Base64<RuntimeSpec> = match CodecBuilder::from_table([b'A'; 64]) {
    Ok(builder) => match builder.build() {
        Ok(codec) => codec,
        Err(_) => panic!("invalid const codec policy"),
    },
    Err(_) => panic!("invalid const duplicate alphabet"),
};

fn main() {}
RS

cat >"$case_dir/wrong-encode-size.rs" <<'RS'
use base64_ng::STRICT_STANDARD_PADDED;

const _: [u8; 7] = match STRICT_STANDARD_PADDED.encode_array(b"hello") {
    Ok(output) => output,
    Err(_) => panic!("invalid const encode output size"),
};

fn main() {}
RS

cat >"$case_dir/wrong-decode-size.rs" <<'RS'
use base64_ng::STRICT_STANDARD_PADDED;

const _: [u8; 6] = match STRICT_STANDARD_PADDED.decode_array(b"aGVsbG8=") {
    Ok(output) => output,
    Err(_) => panic!("invalid const decode output size"),
};

fn main() {}
RS

cat >"$case_dir/secret-clone.rs" <<'RS'
use base64_ng::SecretArray;

fn main() {
    let secret = SecretArray::<3>::from_array(*b"key", 3).unwrap();
    let _duplicate = secret.clone();
}
RS

check_case() {
    source="$1"
    cp "$case_dir/$source.rs" "$source_dir/main.rs"
    run_cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml"
}

compile_failure() {
    source="$1"
    expected="$2"
    log="$workdir/$source.log"
    cp "$case_dir/$source.rs" "$source_dir/main.rs"
    if run_cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" >"$log" 2>&1; then
        echo "2.0 const buffers: invalid case compiled: $source" >&2
        exit 1
    fi
    if ! grep -F -q "$expected" "$log"; then
        echo "2.0 const buffers: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

check_case valid
compile_failure invalid-literal 'invalid const base64 literal'
compile_failure invalid-padding 'invalid const base64 padding'
compile_failure invalid-alphabet 'invalid const duplicate alphabet'
compile_failure wrong-encode-size 'invalid const encode output size'
compile_failure wrong-decode-size 'invalid const decode output size'
compile_failure secret-clone 'no method named `clone`'

run_cargo test --lib 'v2::const_buffer_tests'
run_cargo test --release --lib 'v2::const_buffer_tests'
run_cargo test --no-default-features --lib 'v2::const_buffer_tests'
run_cargo clippy --all-features --lib --tests -- -D warnings

cp "$case_dir/valid.rs" "$source_dir/main.rs"
run_cargo build --quiet --release --offline --manifest-path "$workdir/Cargo.toml"
binary="$workdir/target/release/base64-ng-2-0-const-buffer-smoke"
pointer_bits="$(run_rustc --print cfg | sed -n 's/^target_pointer_width="\([0-9][0-9]*\)"$/\1/p')"
pointer_bytes="$((pointer_bits / 8))"
{
    echo "base64-ng 2.0 const/bounded resource evidence"
    echo "toolchain=$(run_rustc -Vv | sed -n 's/^release: /rustc-/p')"
    echo "target=$(run_rustc -Vv | sed -n 's/^host: //p')"
    echo "encoded_array_64_bytes=$((64 + pointer_bytes))"
    echo "decoded_array_256_bytes=$((256 + pointer_bytes))"
    echo "secret_array_1024_bytes=$((1024 + pointer_bytes))"
    echo "fixture_binary_bytes=$(wc -c <"$binary" | tr -d ' ')"
    echo "stack_bound=object-size only; complete dynamic call chain not measured"
} >"$evidence_dir/resource-shapes.txt"

echo "2.0 const buffers: const diagnostics, bounded invariants, and resource shapes ok"
