#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_ALPHABET_TOOLCHAIN:-}"
workdir="target/2_0_alphabet_compile"
mkdir -p "$workdir"

test -s docs/2.0_VALIDATED_ALPHABETS.md
for required in \
    'exactly one owned `[u8; 64]`' \
    'no function pointer, trait object, callback, or overridable mapping method' \
    'all 2,016 duplicate-position pairs' \
    'Secret decode and Commit 20 secret encode scan all 64 table entries' \
    'validated_alphabet_constructor_indexing_is_bounded'
do
    if ! grep -F -q "$required" docs/2.0_VALIDATED_ALPHABETS.md; then
        echo "2.0 alphabet: contract documentation is missing: $required" >&2
        exit 1
    fi
done

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

cat >"$workdir/valid.rs" <<'RS'
#![allow(dead_code)]

#[path = "../../src/v2/alphabet.rs"]
mod alphabet;

use alphabet::ValidatedAlphabet;

const TABLE: [u8; 64] =
    *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const VALID: ValidatedAlphabet = match ValidatedAlphabet::new(TABLE) {
    Ok(alphabet) => alphabet,
    Err(_) => panic!("valid alphabet rejected"),
};
const ENCODED: Option<u8> = VALID.encode_value(63);
const DECODED: Option<u8> = VALID.decode_byte(b'9');
const FROM_SLICE: ValidatedAlphabet = match ValidatedAlphabet::try_from_slice(&TABLE) {
    Ok(alphabet) => alphabet,
    Err(_) => panic!("valid alphabet slice rejected"),
};

fn main() {
    assert_eq!(ENCODED, Some(b'9'));
    assert_eq!(DECODED, Some(63));
    assert_eq!(FROM_SLICE, VALID);
    assert_eq!(core::mem::size_of::<ValidatedAlphabet>(), 64);
}
RS

cat >"$workdir/invalid-duplicate.rs" <<'RS'
#[path = "../../src/v2/alphabet.rs"]
mod alphabet;

use alphabet::ValidatedAlphabet;

const _: ValidatedAlphabet = match ValidatedAlphabet::new([b'A'; 64]) {
    Ok(alphabet) => alphabet,
    Err(_) => panic!("invalid alphabet constant"),
};

fn main() {}
RS

cat >"$workdir/invalid-padding.rs" <<'RS'
#[path = "../../src/v2/alphabet.rs"]
mod alphabet;

use alphabet::ValidatedAlphabet;

const fn invalid_table() -> [u8; 64] {
    let mut table = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    table[17] = b'=';
    table
}

const _: ValidatedAlphabet = match ValidatedAlphabet::new(invalid_table()) {
    Ok(alphabet) => alphabet,
    Err(_) => panic!("invalid alphabet constant"),
};

fn main() {}
RS

cat >"$workdir/invalid-byte.rs" <<'RS'
#[path = "../../src/v2/alphabet.rs"]
mod alphabet;

use alphabet::ValidatedAlphabet;

const fn invalid_table() -> [u8; 64] {
    let mut table = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    table[23] = b'\n';
    table
}

const _: ValidatedAlphabet = match ValidatedAlphabet::new(invalid_table()) {
    Ok(alphabet) => alphabet,
    Err(_) => panic!("invalid alphabet constant"),
};

fn main() {}
RS

cat >"$workdir/invalid-length.rs" <<'RS'
#[path = "../../src/v2/alphabet.rs"]
mod alphabet;

use alphabet::ValidatedAlphabet;

const _: ValidatedAlphabet = match ValidatedAlphabet::try_from_slice(b"too short") {
    Ok(alphabet) => alphabet,
    Err(_) => panic!("invalid alphabet constant"),
};

fn main() {}
RS

compile_failure() {
    source="$1"
    log="$workdir/$source.log"
    if run_rustc \
        --edition=2024 \
        --crate-name base64_ng_invalid_alphabet \
        "$workdir/$source.rs" \
        --out-dir "$workdir" >"$log" 2>&1
    then
        echo "2.0 alphabet: invalid const alphabet compiled: $source" >&2
        exit 1
    fi
    if ! grep -F -q "invalid alphabet constant" "$log"; then
        echo "2.0 alphabet: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

run_rustc \
    --edition=2024 \
    --crate-name base64_ng_validated_alphabet_const \
    "$workdir/valid.rs" \
    --out-dir "$workdir"
"$workdir/base64_ng_validated_alphabet_const"

for invalid in invalid-duplicate invalid-padding invalid-byte invalid-length; do
    compile_failure "$invalid"
done

if grep -n -F \
    -e unsafe \
    -e 'std::' \
    -e 'alloc::' \
    -e 'Box<' \
    -e 'dyn ' \
    -e 'extern "' \
    -e 'fn(' \
    -e 'fn (' \
    -e 'impl Alphabet' \
    -e 'trait Alphabet' \
    src/v2/alphabet.rs
then
    echo "2.0 alphabet: validated value gained executable or runtime-only state" >&2
    exit 1
fi

run_cargo test --lib 'v2::alphabet_tests'

echo "2.0 alphabet: owned table, const failures, and exhaustive mapping checks ok"
