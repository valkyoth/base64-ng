#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-subtle/Cargo.toml"
source_file="crates/base64-ng-subtle/src/lib.rs"
workdir="target/2_0_subtle"
source_dir="$workdir/src"
case_dir="$workdir/cases"
mkdir -p "$source_dir" "$case_dir"

for required in \
    'pub trait SubtleSecretEq: sealed::Sealed' \
    'fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice' \
    'impl<const CAP: usize> SubtleSecretEq for SecretArray<CAP>' \
    'impl SubtleSecretEq for SecretInput' \
    'impl SubtleSecretEq for SecretOutput' \
    'impl SubtleSecretEq for ExposedSecret' \
    'impl SubtleSecretEq for ExposedSecretMut' \
    'impl SubtleSecretEq for SecretVec'
do
    if ! grep -F -q "$required" "$source_file"; then
        echo "2.0 subtle equality: missing reviewed API marker: $required" >&2
        exit 1
    fi
done

if ! grep -F -q 'subtle = { version = "=2.6.1", default-features = false }' "$manifest"; then
    echo "2.0 subtle equality: subtle dependency must remain exact-pinned to 2.6.1" >&2
    exit 1
fi

if grep -F -q 'fn subtle_verify' "$source_file" || \
   grep -F -q 'impl SubtleSecretEq for DecodedBuffer' "$source_file" || \
   grep -F -q 'impl SubtleSecretEq for EncodedBuffer' "$source_file"; then
    echo "2.0 subtle equality: ambiguous or ordinary-buffer equality sugar remains" >&2
    exit 1
fi

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-subtle-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["secrets"] }
base64-ng-subtle = { path = "../../crates/base64-ng-subtle", default-features = false }
TOML

cat >"$case_dir/valid.rs" <<'RS'
use base64_ng::secret::{SecretArray, SecretInput, SecretOutput};
use base64_ng_subtle::SubtleSecretEq;

fn main() {
    let input = SecretInput::new(b"token");
    let _: bool = input.subtle_ct_eq_public_len(b"token").into();

    let fixed = SecretArray::from_array(*b"token", 5).unwrap();
    let _: bool = fixed.subtle_ct_eq_public_len(b"token").into();

    let mut storage = *b"tokenxxx";
    let output = SecretOutput::from_initialized(&mut storage, 5).unwrap();
    let _: bool = output.subtle_ct_eq_public_len(b"token").into();
}
RS

cat >"$case_dir/partial-eq.rs" <<'RS'
use base64_ng::secret::SecretArray;

fn main() {
    let left = SecretArray::from_array(*b"token", 5).unwrap();
    let right = SecretArray::from_array(*b"token", 5).unwrap();
    let _ = left == right;
}
RS

cat >"$case_dir/boolean-sugar.rs" <<'RS'
use base64_ng::secret::SecretArray;
use base64_ng_subtle::SubtleSecretEq;

fn main() {
    let secret = SecretArray::from_array(*b"token", 5).unwrap();
    let _ = secret.subtle_verify(b"token");
}
RS

cat >"$case_dir/ordinary-buffer.rs" <<'RS'
use base64_ng::STANDARD;
use base64_ng_subtle::SubtleSecretEq;

fn main() {
    let ordinary = STANDARD.decode_buffer::<5>(b"aGVsbG8=").unwrap();
    let _ = ordinary.subtle_ct_eq_public_len(b"hello");
}
RS

check_case() {
    cp "$case_dir/$1.rs" "$source_dir/main.rs"
    cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml"
}

compile_failure() {
    source="$1"
    expected="$2"
    log="$workdir/$source.log"
    cp "$case_dir/$source.rs" "$source_dir/main.rs"
    if cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" >"$log" 2>&1; then
        echo "2.0 subtle equality: invalid case compiled: $source" >&2
        exit 1
    fi
    if ! grep -F -q "$expected" "$log"; then
        echo "2.0 subtle equality: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

check_case valid
compile_failure partial-eq 'binary operation `==` cannot be applied'
compile_failure boolean-sugar 'no method named `subtle_verify`'
compile_failure ordinary-buffer 'no method named `subtle_ct_eq_public_len`'

echo "2.0 subtle equality: no-default-features"
cargo test --manifest-path "$manifest" --no-default-features

echo "2.0 subtle equality: all features"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 subtle equality: release"
cargo test --manifest-path "$manifest" --all-features --release

echo "2.0 subtle equality: lint"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings

echo "2.0 subtle equality: documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 subtle equality: locked dudect harness"
cargo check --locked --manifest-path dudect/Cargo.toml --bins
cargo test --locked --manifest-path dudect/Cargo.toml --bins

echo "2.0 subtle equality: optimized assembly"
BASE64_NG_ALLOW_DIRTY_EVIDENCE=1 scripts/generate_subtle_asm_evidence.sh

echo "2.0 subtle equality: sealed explicit comparison boundary ok"
