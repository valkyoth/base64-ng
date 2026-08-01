#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_SECRET_ENCODER_TOOLCHAIN:-}"
workdir="target/2_0_secret_encoder"
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
    'Built-in alphabets use private arithmetic mapping.' \
    'Custom alphabets use one fixed 64-entry scan per output symbol.' \
    'Ordinary formatters and counted sinks do not accept `SecretInput`.' \
    'Encoded secret output remains secret until explicit exposure or declassification.' \
    'No secret frame reallocates after accepting input.'
do
    if ! grep -F -q "$required" docs/2.0_SECRET_ENCODING.md; then
        echo "2.0 secret encoder: documentation is missing: $required" >&2
        exit 1
    fi
done

for forbidden in \
    'encode_backend' \
    'encode_into('
do
    if grep -F -q "$forbidden" src/v2/secret_encoder.rs; then
        echo "2.0 secret encoder: ordinary encode routing found: $forbidden" >&2
        exit 1
    fi
done

if grep -F -q 'as_array()[usize::from(value)]' src/v2/secret_encoder.rs; then
    echo "2.0 secret encoder: secret-indexed alphabet lookup found" >&2
    exit 1
fi

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-secret-encoder-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["secrets"] }
TOML

cat >"$case_dir/valid.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, secret::{SecretArrayEncoder, SecretInput}};

fn main() {
    let input = SecretInput::new(b"secret");
    let encoded = STRICT_STANDARD_PADDED
        .encode_secret_array::<8>(&input)
        .unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"c2VjcmV0");

    let mut frame = SecretArrayEncoder::<8>::new(&STRICT_STANDARD_PADDED, 6).unwrap();
    frame.update(&SecretInput::new(b"sec")).unwrap();
    frame.update(&SecretInput::new(b"ret")).unwrap();
    let declassified = frame.finish().unwrap().declassify();
    assert_eq!(declassified.as_bytes(), b"c2VjcmV0");
}
RS

cat >"$case_dir/ordinary-incremental.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, secret::SecretInput};

fn main() {
    let mut encoder = STRICT_STANDARD_PADDED.encoder();
    let input = SecretInput::new(b"secret");
    let mut output = [0u8; 8];
    let _ = encoder.update(&input, &mut output);
}
RS

cat >"$case_dir/ordinary-formatter.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, secret::SecretInput};

struct Sink;

impl core::fmt::Write for Sink {
    fn write_str(&mut self, _: &str) -> core::fmt::Result {
        Ok(())
    }
}

fn main() {
    let input = SecretInput::new(b"secret");
    let _ = STRICT_STANDARD_PADDED.encode_to_fmt(&input, &mut Sink);
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
        echo "2.0 secret encoder: classified input reached ordinary surface: $source" >&2
        exit 1
    fi
    if ! grep -F -q 'mismatched types' "$log"; then
        echo "2.0 secret encoder: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

check_case valid
compile_failure ordinary-incremental
compile_failure ordinary-formatter

run_cargo test --no-default-features --features secrets --lib 'v2::secret_encoder_tests'
run_cargo test --all-features --lib 'v2::secret_encoder_tests'
run_cargo test --all-features --test v2_secret_encoder
run_cargo clippy --no-default-features --features secrets --lib --tests -- -D warnings
run_cargo clippy --all-features --lib --tests -- -D warnings

echo "2.0 secret encoder: bounded fixed-work frame evidence ok"
