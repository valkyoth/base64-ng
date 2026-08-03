#!/usr/bin/env sh
set -eu

root="$(pwd)"
workdir="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-derive.XXXXXX")"
trap 'rm -rf "$workdir"' EXIT HUP INT TERM
mkdir -p "$workdir/src"

cat >"$workdir/Cargo.toml" <<EOF
[package]
name = "base64-ng-derive-contract"
version = "0.0.0"
edition = "2024"

[dependencies]
base64-ng = { path = "$root", default-features = false, features = ["secrets"] }
base64-ng-derive = { path = "$root/crates/base64-ng-derive" }

[workspace]
EOF

compile_failure() {
    name="$1"
    expected="$2"
    source="$3"
    printf '%s\n' "$source" >"$workdir/src/main.rs"
    if cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" \
        >"$workdir/$name.log" 2>&1; then
        echo "2.0 derive: invalid case compiled: $name" >&2
        exit 1
    fi
    if ! grep -F -q "$expected" "$workdir/$name.log"; then
        echo "2.0 derive: $name failed for an unexpected reason" >&2
        cat "$workdir/$name.log" >&2
        exit 1
    fi
}

base='use base64_ng::secret::SecretArray;
use base64_ng_derive::Base64Secret;'

compile_failure missing-policy \
    'Base64Secret: requires exactly one' \
    "$base
#[derive(Base64Secret)]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure missing-exposure \
    'Base64Secret: missing required policy key `exposure`' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"standard\", padding = \"padded\", exact_length = 32)]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure duplicate-key \
    'Base64Secret: duplicate policy key `alphabet`' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"standard\", alphabet = \"url_safe\", padding = \"padded\", exact_length = 32, exposure = \"none\")]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure unknown-key \
    'Base64Secret: unknown policy key `mode`' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"standard\", padding = \"padded\", exact_length = 32, exposure = \"none\", mode = \"strict\")]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure invalid-alphabet \
    'Base64Secret: `alphabet` must be "standard" or "url_safe"' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"crypt\", padding = \"padded\", exact_length = 32, exposure = \"none\")]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure invalid-padding \
    'Base64Secret: `padding` must be "padded" or "unpadded"' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"standard\", padding = \"optional\", exact_length = 32, exposure = \"none\")]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure invalid-exposure \
    'Base64Secret: `exposure` must be "none", "read", or "read_write"' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"standard\", padding = \"padded\", exact_length = 32, exposure = \"implicit\")]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure length-mismatch \
    'Base64Secret: `exact_length = 31` does not match `SecretArray<32>`' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"standard\", padding = \"padded\", exact_length = 31, exposure = \"none\")]
struct Key(base64_ng::secret::SecretArray<32>);
fn main() {}"

compile_failure raw-array \
    'Base64Secret: field must be `base64_ng::secret::SecretArray<N>`' \
    'use base64_ng_derive::Base64Secret;
#[derive(Base64Secret)]
#[base64_ng(alphabet = "standard", padding = "padded", exact_length = 32, exposure = "none")]
struct Key([u8; 32]);
fn main() {}'

compile_failure public-field \
    'Base64Secret: the `SecretArray<N>` tuple field must be private' \
    "$base
#[derive(Base64Secret)]
#[base64_ng(alphabet = \"standard\", padding = \"padded\", exact_length = 32, exposure = \"none\")]
struct Key(pub base64_ng::secret::SecretArray<32>);
fn main() {}"

policy='#[base64_ng(alphabet = "standard", padding = "padded", exact_length = 3, exposure = "none")]'
compile_failure implicit-as-ref \
    'the trait bound `Key: AsRef<[u8]>` is not satisfied' \
    "$base
#[derive(Base64Secret)]
$policy
struct Key(base64_ng::secret::SecretArray<3>);
fn main() {
    fn accepts<T: AsRef<[u8]>>(_: &T) {}
    let _ = accepts as fn(&Key);
}"

compile_failure clone \
    'no method named `clone`' \
    "$base
#[derive(Base64Secret)]
$policy
struct Key(base64_ng::secret::SecretArray<3>);
fn main() {
    let key = Key::decode_base64(&base64_ng::secret::SecretInput::new(b\"a2V5\")).unwrap();
    let _ = key.clone();
}"

compile_failure ordinary-equality \
    'binary operation `==` cannot be applied to type `Key`' \
    "$base
#[derive(Base64Secret)]
$policy
struct Key(base64_ng::secret::SecretArray<3>);
fn main() {
    let left = Key::decode_base64(&base64_ng::secret::SecretInput::new(b\"a2V5\")).unwrap();
    let right = Key::decode_base64(&base64_ng::secret::SecretInput::new(b\"a2V5\")).unwrap();
    let _ = left == right;
}"

compile_failure from-str \
    'the trait bound `Key: FromStr` is not satisfied' \
    "$base
#[derive(Base64Secret)]
$policy
struct Key(base64_ng::secret::SecretArray<3>);
fn main() { let _: Key = \"a2V5\".parse().unwrap(); }"

compile_failure no-exposure \
    'no method named `expose_secret`' \
    "$base
#[derive(Base64Secret)]
$policy
struct Key(base64_ng::secret::SecretArray<3>);
fn main() {
    let key = Key::decode_base64(&base64_ng::secret::SecretInput::new(b\"a2V5\")).unwrap();
    let _ = key.expose_secret();
}"

echo "2.0 derive: companion tests"
cargo test --quiet --offline --manifest-path crates/base64-ng-derive/Cargo.toml

for forbidden in \
    'base64_ng::ct::' \
    'base64_ng::clear_bytes' \
    'impl ::core::convert::AsRef' \
    'impl ::core::convert::AsMut' \
    'impl ::core::str::FromStr' \
    'into_secret_array' \
    'constant_time_eq'
do
    if grep -F -q "$forbidden" crates/base64-ng-derive/src/expand.rs; then
        echo "2.0 derive: generated implementation contains forbidden surface: $forbidden" >&2
        exit 1
    fi
done

for required in \
    'SecretArrayFrame' \
    'SecretArrayEncoder' \
    'SecretInput' \
    'ExposedSecret' \
    'ExposedSecretMut'
do
    if ! grep -F -q "$required" crates/base64-ng-derive/src/expand.rs; then
        echo "2.0 derive: generated implementation is missing: $required" >&2
        exit 1
    fi
done

echo "2.0 derive: sealed codec, staged lifecycle, exposure, and diagnostics ok"
