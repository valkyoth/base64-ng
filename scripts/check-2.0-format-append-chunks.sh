#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_FORMAT_TOOLCHAIN:-}"
workdir="target/2_0_format_lifetime"
mkdir -p "$workdir/src"

run_cargo() {
    if [ -n "$toolchain" ]; then
        cargo +"$toolchain" "$@"
    else
        cargo "$@"
    fi
}

test -s docs/2.0_FORMAT_APPEND_CHUNKS.md
for required in \
    '`Base64::display' \
    '`Base64::encode_to_fmt' \
    '`Base64::encode_to_counted' \
    '`Base64::encode_append' \
    '`Base64::decode_append' \
    '`Base64::encoded_chunks' \
    '`write_str` call partially changed its sink' \
    'This is not zero-copy' \
    'panic restores the entry length'
do
    if ! grep -F -q "$required" docs/2.0_FORMAT_APPEND_CHUNKS.md; then
        echo "2.0 format/append/chunks: documentation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub struct EncodedChunk' \
    'pub struct EncodedChunks' \
    'pub fn encoded_chunks'
do
    grep -F -q "$required" src/v2/chunks.rs
done

for required in \
    'pub struct EncodedDisplay' \
    'pub trait CountedSink' \
    'pub enum CountedWriteError' \
    'pub fn display' \
    'pub fn encode_to_fmt' \
    'pub fn encode_to_counted'
do
    grep -F -q "$required" src/v2/formatting.rs
done

for required in \
    'pub fn encode_append' \
    'pub fn decode_append' \
    'impl Drop for StringRollback' \
    'impl Drop for VecRollback'
do
    grep -F -q "$required" src/v2/append.rs
done

run_cargo test --lib 'v2::chunk_tests'
run_cargo test --lib 'v2::formatting_tests'
run_cargo test --lib 'v2::append_tests'
run_cargo test --test v2_formatting_alloc
run_cargo test --release --lib 'v2::chunk_tests'
run_cargo test --release --lib 'v2::formatting_tests'
run_cargo test --release --lib 'v2::append_tests'
run_cargo test --release --test v2_formatting_alloc
run_cargo test --no-default-features --lib 'v2::chunk_tests'
run_cargo test --no-default-features --lib 'v2::formatting_tests'
scripts/check-2.0-migration-smoke.sh
run_cargo test --offline --manifest-path target/2_0_migration_smoke/Cargo.toml \
    format_append_and_chunks_2_0_surface_is_public_and_external

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-format-lifetime"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../.." }
TOML

cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::{EncodedDisplay, STRICT_STANDARD_PADDED};

fn invalid_escape() -> EncodedDisplay<'static> {
    let input = vec![b's', b'e', b'c', b'r', b'e', b't'];
    STRICT_STANDARD_PADDED.display(&input).unwrap()
}

fn main() {
    let _ = invalid_escape();
}
RS

if run_cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" \
    >"$workdir/lifetime.log" 2>&1
then
    echo "2.0 format/append/chunks: borrowed display escaped its input" >&2
    exit 1
fi
if ! grep -F -q 'E0515' "$workdir/lifetime.log"; then
    echo "2.0 format/append/chunks: lifetime case failed unexpectedly" >&2
    cat "$workdir/lifetime.log" >&2
    exit 1
fi

run_cargo clippy --all-features --lib --tests -- -D warnings
run_cargo clippy --no-default-features --lib --tests -- -D warnings

echo "2.0 format/append/chunks: allocation, progress, rollback, chunks, and lifetime ok"
