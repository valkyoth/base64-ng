#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_LINE_WRAP_TOOLCHAIN:-}"
workdir="target/2_0_line_wrap_compile"
mkdir -p "$workdir"

test -s docs/2.0_LINE_WRAPPING.md
for required in \
    'line width is a `NonZeroUsize`' \
    '`LineWrap::try_new` is the only runtime constructor' \
    '`BodyWrap`' \
    '`BodyLineEnding`' \
    '`BodyWrapError`' \
    'there is no `is_valid` method' \
    'MIME content-transfer body layout only' \
    'PEM body layout only' \
    '`usize::MAX`' \
    'leave the caller'
do
    if ! grep -F -q "$required" docs/2.0_LINE_WRAPPING.md; then
        echo "2.0 line wrapping: contract documentation is missing: $required" >&2
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

#[path = "../../src/v2/wrapping.rs"]
mod wrapping;

use wrapping::{LineEnding, LineWrap};

const VALID: LineWrap = match LineWrap::try_new(76, LineEnding::CrLf) {
    Ok(wrap) => wrap,
    Err(_) => panic!("valid line wrapping rejected"),
};

fn main() {
    assert_eq!(VALID.line_width().get(), 76);
    assert_eq!(VALID.line_ending(), LineEnding::CrLf);
    assert_eq!(VALID.checked_output_len(77), Some(79));
    assert_eq!(VALID, LineWrap::MIME_BODY_WRAP);
}
RS

cat >"$workdir/private-fields.rs" <<'RS'
#[path = "../../src/v2/wrapping.rs"]
mod wrapping;

use core::num::NonZeroUsize;
use wrapping::{LineEnding, LineWrap};

const _: LineWrap = LineWrap {
    line_width: NonZeroUsize::MIN,
    line_ending: LineEnding::Lf,
};

fn main() {}
RS

cat >"$workdir/zero-width.rs" <<'RS'
#[path = "../../src/v2/wrapping.rs"]
mod wrapping;

use wrapping::{LineEnding, LineWrap};

const _: LineWrap = match LineWrap::try_new(0, LineEnding::Lf) {
    Ok(wrap) => wrap,
    Err(_) => panic!("zero-width wrapping rejected"),
};

fn main() {}
RS

compile_failure() {
    source="$1"
    expected="$2"
    log="$workdir/$source.log"
    if run_rustc \
        --edition=2024 \
        --crate-name base64_ng_invalid_line_wrap \
        "$workdir/$source.rs" \
        --out-dir "$workdir" >"$log" 2>&1
    then
        echo "2.0 line wrapping: invalid source compiled: $source" >&2
        exit 1
    fi
    if ! grep -F -q "$expected" "$log"; then
        echo "2.0 line wrapping: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

run_rustc \
    --edition=2024 \
    --crate-name base64_ng_valid_line_wrap \
    "$workdir/valid.rs" \
    --out-dir "$workdir"
"$workdir/base64_ng_valid_line_wrap"

compile_failure private-fields 'private field'
compile_failure zero-width 'zero-width wrapping rejected'

if grep -n -F \
    -e unsafe \
    -e 'alloc::' \
    -e 'Box<' \
    -e 'pub(crate) fn new' \
    -e 'pub(crate) const fn new' \
    -e 'checked_new' \
    -e 'is_valid' \
    src/v2/wrapping.rs
then
    echo "2.0 line wrapping: invariant model gained forbidden state or API" >&2
    exit 1
fi

if [ "$(grep -F -c 'std::' src/v2/wrapping.rs)" -ne 1 ] ||
    ! grep -F -q 'impl std::error::Error for LineWrapError {}' src/v2/wrapping.rs
then
    echo "2.0 line wrapping: unexpected std-only surface" >&2
    exit 1
fi

for public_alias in \
    'LineEnding as BodyLineEnding' \
    'LineWrap as BodyWrap' \
    'LineWrapError as BodyWrapError'
do
    if ! grep -F -q "$public_alias" src/v2/mod.rs; then
        echo "2.0 line wrapping: missing public body alias: $public_alias" >&2
        exit 1
    fi
done

if ! grep -F -q 'line_width: NonZeroUsize' src/v2/wrapping.rs; then
    echo "2.0 line wrapping: width is no longer stored as NonZeroUsize" >&2
    exit 1
fi

run_cargo test --lib 'v2::wrapping_tests'
run_cargo clippy --lib --all-features -- -D warnings

echo "2.0 line wrapping: non-zero policy and LF/CRLF evidence ok"
