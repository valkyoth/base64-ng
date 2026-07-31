#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_SECRET_STORAGE_TOOLCHAIN:-}"
workdir="target/2_0_secret_storage"
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
    'Secret owners and classified input implement no `AsRef`, `AsMut`, or `Deref`' \
    '`SecretOutput::from_initialized` wipes unused tail bytes' \
    '`declassify_into_unprotected_vec`' \
    'destructors do not run after `mem::forget`'
do
    if ! grep -F -q "$required" docs/2.0_SECRET_STORAGE_AND_EXPOSURE.md; then
        echo "2.0 secret storage: documentation is missing: $required" >&2
        exit 1
    fi
done

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-secret-storage-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["secrets"] }
base64-ng-derive = { path = "../../crates/base64-ng-derive" }
TOML

cat >"$case_dir/valid.rs" <<'RS'
use base64_ng::{
    STRICT_STANDARD_PADDED,
    secret::{SecretArray, SecretInput, SecretOutput},
};

fn main() {
    let input = SecretInput::new(b"secret");
    let mut encoded = [0u8; 8];
    let written = STRICT_STANDARD_PADDED
        .encode_into(input.expose_secret().as_bytes(), &mut encoded)
        .unwrap();
    assert_eq!(written, 8);

    let fixed = SecretArray::from_array(*b"keyxxxxx", 3).unwrap();
    assert_eq!(AsRef::<[u8]>::as_ref(&fixed.expose_secret()), b"key");

    let mut output_bytes = [0u8; 8];
    let mut output = SecretOutput::empty(&mut output_bytes);
    assert!(AsMut::<[u8]>::as_mut(&mut output.expose_secret_mut()).is_empty());
}
RS

cat >"$case_dir/implicit-input.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, secret::SecretInput};

fn main() {
    let input = SecretInput::new(b"secret");
    let mut output = [0u8; 8];
    let _ = STRICT_STANDARD_PADDED.encode_into(input, &mut output);
}
RS

cat >"$case_dir/secret-array-as-ref.rs" <<'RS'
use base64_ng::secret::SecretArray;

fn needs_bytes<T: AsRef<[u8]>>(_: &T) {}

fn main() {
    let secret = SecretArray::from_array(*b"key", 3).unwrap();
    needs_bytes(&secret);
}
RS

cat >"$case_dir/secret-output-as-mut.rs" <<'RS'
use base64_ng::secret::SecretOutput;

fn needs_bytes<T: AsMut<[u8]>>(_: &mut T) {}

fn main() {
    let mut bytes = [0u8; 8];
    let mut secret = SecretOutput::empty(&mut bytes);
    needs_bytes(&mut secret);
}
RS

cat >"$case_dir/secret-clone.rs" <<'RS'
use base64_ng::secret::{SecretArray, SecretInput};

fn main() {
    let input = SecretInput::new(b"secret");
    let _ = input.clone();
    let fixed = SecretArray::from_array(*b"key", 3).unwrap();
    let _ = fixed.clone();
}
RS

cat >"$case_dir/derive-as-ref.rs" <<'RS'
use base64_ng_derive::Base64Secret;

#[derive(Base64Secret)]
struct Key([u8; 3]);

fn main() {
    let key = Key::from(*b"key");
    let _ = AsRef::<[u8]>::as_ref(&key);
}
RS

check_case() {
    cp "$case_dir/$1.rs" "$source_dir/main.rs"
    run_cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml"
}

compile_failure() {
    source="$1"
    expected="$2"
    log="$workdir/$source.log"
    cp "$case_dir/$source.rs" "$source_dir/main.rs"
    if run_cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" >"$log" 2>&1; then
        echo "2.0 secret storage: invalid case compiled: $source" >&2
        exit 1
    fi
    if ! grep -F -q "$expected" "$log"; then
        echo "2.0 secret storage: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

check_case valid
compile_failure implicit-input 'mismatched types'
compile_failure secret-array-as-ref 'the trait bound `SecretArray<3>: AsRef<[u8]>` is not satisfied'
compile_failure secret-output-as-mut 'the trait bound `SecretOutput'
compile_failure secret-clone 'no method named `clone`'
compile_failure derive-as-ref 'the trait bound `Key: AsRef<[u8]>` is not satisfied'

run_cargo test --no-default-features --features secrets --lib 'v2::secret_storage_tests'
run_cargo test --features secrets --lib 'v2::secret_storage_tests'
run_cargo test --all-features --test v2_secret_storage
run_cargo test --manifest-path crates/base64-ng-derive/Cargo.toml
run_cargo clippy --all-features --lib --tests -- -D warnings
run_cargo clippy --manifest-path crates/base64-ng-derive/Cargo.toml --all-targets -- -D warnings

echo "2.0 secret storage: explicit exposure and cleanup evidence ok"
