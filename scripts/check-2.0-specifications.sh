#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_SPECIFICATION_TOOLCHAIN:-}"
workdir="target/2_0_specification_compile"
mkdir -p "$workdir"

test -s docs/2.0_CODEC_SPECIFICATIONS.md
for required in \
    'One sealed `Codec` trait' \
    'zero-sized marker' \
    'no allocation' \
    'padding-indifferent' \
    'noncanonical trailing bits' \
    '`base64` `0.23.0`'
do
    if ! grep -F -q "$required" docs/2.0_CODEC_SPECIFICATIONS.md; then
        echo "2.0 specifications: contract documentation is missing: $required" >&2
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

write_prelude() {
    target="$1"
    cat >"$target" <<'RS'
#![allow(dead_code)]

#[path = "../../src/v2/alphabet.rs"]
mod alphabet;
#[path = "../../src/v2/specifications.rs"]
mod specifications;
RS
}

write_prelude "$workdir/valid.rs"
cat >>"$workdir/valid.rs" <<'RS'
use specifications::{Codec, CodecBuilder, DecodePadding, STRICT_STANDARD_PADDED};

const TABLE: [u8; 64] =
    *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const CUSTOM: specifications::Base64<specifications::RuntimeSpec> =
    match CodecBuilder::from_table(TABLE) {
        Ok(builder) => match builder
            .decode_padding(DecodePadding::Indifferent)
            .build()
        {
            Ok(codec) => codec,
            Err(_) => panic!("valid policy rejected"),
        },
        Err(_) => panic!("valid alphabet rejected"),
    };

fn object_safe(codec: &dyn Codec) -> specifications::CodecSettings {
    codec.settings()
}

fn main() {
    assert_eq!(core::mem::size_of_val(STRICT_STANDARD_PADDED.specification()), 0);
    assert_eq!(CUSTOM.settings().alphabet().as_array(), &TABLE);
    assert_eq!(
        object_safe(STRICT_STANDARD_PADDED.specification()),
        STRICT_STANDARD_PADDED.settings()
    );
}
RS

write_prelude "$workdir/unsealed.rs"
cat >>"$workdir/unsealed.rs" <<'RS'
use specifications::{Codec, CodecSettings};

struct External;

impl Codec for External {
    fn settings(&self) -> CodecSettings {
        panic!("must not compile")
    }
}

fn main() {}
RS

write_prelude "$workdir/mutate.rs"
cat >>"$workdir/mutate.rs" <<'RS'
use specifications::{Codec, DecodePadding, STRICT_STANDARD_PADDED};

fn main() {
    let mut settings = STRICT_STANDARD_PADDED.settings();
    settings.decode_padding = DecodePadding::Indifferent;
}
RS

compile_failure() {
    source="$1"
    expected="$2"
    log="$workdir/$source.log"
    if run_rustc \
        --edition=2024 \
        --crate-name "base64_ng_invalid_specification_$source" \
        "$workdir/$source.rs" \
        --out-dir "$workdir" >"$log" 2>&1
    then
        echo "2.0 specifications: invalid source compiled: $source" >&2
        exit 1
    fi
    if ! grep -F -q "$expected" "$log"; then
        echo "2.0 specifications: $source failed for an unexpected reason" >&2
        cat "$log" >&2
        exit 1
    fi
}

run_rustc \
    --edition=2024 \
    --crate-name base64_ng_valid_specification \
    "$workdir/valid.rs" \
    --out-dir "$workdir"
"$workdir/base64_ng_valid_specification"

compile_failure unsealed 'the trait bound `External: Sealed` is not satisfied'
compile_failure mutate 'field `decode_padding` of struct `CodecSettings` is private'

if rg -n -F \
    -e unsafe \
    -e 'std::' \
    -e 'alloc::' \
    -e 'Box<' \
    -e 'extern "' \
    src/v2/specifications.rs
then
    echo "2.0 specifications: model gained unsafe or runtime-only state" >&2
    exit 1
fi

run_cargo test --lib 'v2::specification_tests'
run_cargo clippy --lib --all-features -- -D warnings
perf_rustflags="${RUSTFLAGS:-}"
if [ -n "$perf_rustflags" ]; then
    perf_rustflags="$perf_rustflags --cfg base64_ng_perf_evidence"
else
    perf_rustflags="--cfg base64_ng_perf_evidence"
fi
RUSTFLAGS="$perf_rustflags" run_cargo test \
    --manifest-path perf/Cargo.toml 'v2_model::tests'
RUSTFLAGS="$perf_rustflags" run_cargo clippy \
    --manifest-path perf/Cargo.toml --all-targets -- -D warnings

echo "2.0 specifications: sealed presets, builders, and policy evidence ok"
