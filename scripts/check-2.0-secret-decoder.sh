#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_SECRET_DECODER_TOOLCHAIN:-}"
workdir="target/2_0_secret_decoder"
source_dir="$workdir/src"
case_dir="$workdir/cases"
mkdir -p "$source_dir" "$case_dir"

run_cargo() {
    if [ -n "$toolchain" ]; then
        cargo +"$toolchain" "$@"
    else
        cargo "$@"
    fi
}

for required in \
    'The result gate ends the fixed-work validity boundary.' \
    'Every accepted encoded byte performs one complete 64-entry alphabet scan.' \
    'No secret frame reallocates after' \
    'Private staging, encoded input, and final output must be byte-disjoint.' \
    '`InputTooLarge` is decided from public lengths before symbol scanning'
do
    if ! grep -F -q "$required" docs/2.0_SECRET_DECODING.md; then
        echo "2.0 secret decoder: documentation is missing: $required" >&2
        exit 1
    fi
done

for forbidden in \
    'decode_byte(' \
    'decode_backend'
do
    if grep -F -q "$forbidden" src/v2/secret_decoder.rs; then
        echo "2.0 secret decoder: ordinary decode routing found: $forbidden" >&2
        exit 1
    fi
done

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-secret-decoder-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["secrets"] }
TOML

cat >"$case_dir/valid.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, secret::{SecretArrayFrame, SecretInput}};

fn main() {
    let mut frame = SecretArrayFrame::<32>::new(&STRICT_STANDARD_PADDED).unwrap();
    frame.update(&SecretInput::new(b"c2VjcmV0")).unwrap();
    assert_eq!(frame.finish().unwrap().expose_secret().as_bytes(), b"secret");
}
RS

cat >"$case_dir/oversized-direct.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, secret::SecretArrayFrame};

fn main() {
    let _ = SecretArrayFrame::<1025>::new(&STRICT_STANDARD_PADDED);
}
RS

cat >"$case_dir/oversized-macro.rs" <<'RS'
use base64_ng::STRICT_STANDARD_PADDED;

fn main() {
    let _ = base64_ng::secret_array_frame!(STRICT_STANDARD_PADDED, 1025);
}
RS

cat >"$case_dir/oversized-generic.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, secret::SecretArrayFrame};

fn construct<const N: usize>() {
    let _ = SecretArrayFrame::<N>::new(&STRICT_STANDARD_PADDED);
}

fn main() {
    construct::<1025>();
}
RS

check_case() {
    cp "$case_dir/$1.rs" "$source_dir/main.rs"
    run_cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml"
}

compile_failure() {
    source="$1"
    log="$workdir/$source.log"
    cp "$case_dir/$source.rs" "$source_dir/main.rs"
    if run_cargo build --quiet --offline --manifest-path "$workdir/Cargo.toml" >"$log" 2>&1; then
        echo "2.0 secret decoder: oversized case compiled: $source" >&2
        exit 1
    fi
    if ! grep -F -q 'SecretArrayFrame decoded capacity exceeds 1024-byte stack limit' "$log"; then
        echo "2.0 secret decoder: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

check_case valid
compile_failure oversized-direct
compile_failure oversized-macro
compile_failure oversized-generic

run_cargo test --no-default-features --features secrets --lib 'v2::secret_decoder_tests'
run_cargo test --all-features --lib 'v2::secret_decoder_tests'
run_cargo test --all-features --test v2_secret_decoder
run_cargo clippy --no-default-features --features secrets --lib --tests -- -D warnings
run_cargo clippy --all-features --lib --tests -- -D warnings

echo "2.0 secret decoder: bounded fixed-work frame evidence ok"
