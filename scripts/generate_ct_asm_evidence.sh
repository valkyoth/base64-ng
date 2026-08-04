#!/usr/bin/env sh
set -eu

target="${BASE64_NG_CT_ASM_TARGET:-}"
if [ -n "$target" ]; then
    target_triple="$target"
    output_dir="target/release-evidence/asm/$target"
    target_mode="explicit-cross-or-native-target"
else
    target_triple="$(rustc -vV | sed -n 's/^host: //p')"
    output_dir="target/release-evidence/asm"
    target_mode="active-host"
fi
manifest="$output_dir/MANIFEST.txt"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-ct-asm.XXXXXX")"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
. scripts/ct-asm-symbols.sh
evidence_capture_source "ct asm evidence"

copy_single_asm() {
    target_dir="$1"
    output_file="$2"
    set -- "$target_dir"/base64_ng-*.s

    if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
        echo "ct asm evidence: expected exactly one fresh assembly file under $target_dir" >&2
        exit 1
    fi

    cp "$1" "$output_file"
    test -s "$output_file"
}

require_symbol() {
    assembly_file="$1"
    symbol_len="$2"
    symbol_name="$3"
    if ct_asm_symbol_is_defined \
        "$assembly_file" \
        "$symbol_len" \
        "$symbol_name"
    then
        return
    fi

    echo "ct asm evidence: missing non-inlined ${symbol_name} symbol in $assembly_file" >&2
    exit 1
}

release_deps_dir() {
    target_dir="$1"
    if [ -n "$target" ]; then
        printf '%s' "$target_dir/$target/release/deps"
    else
        printf '%s' "$target_dir/release/deps"
    fi
}

require_reviewed_symbols() {
    assembly_file="$1"
    require_symbol "$assembly_file" "10" "wipe_bytes"
    require_symbol "$assembly_file" "12" "wipe_barrier"
    require_symbol "$assembly_file" "27" "constant_time_eq_public_len"
    require_symbol "$assembly_file" "16" "ct_accumulate_u8"
    require_symbol "$assembly_file" "21" "ct_error_gate_barrier"
    require_symbol "$assembly_file" "19" "secret_encode_ascii"
    require_symbol "$assembly_file" "18" "secret_encode_scan"
    require_symbol "$assembly_file" "13" "decode_symbol"
}

wipe_revision="$(sed -n 's/^pub(crate) const WIPE_PRIMITIVE_REVISION: usize = \([0-9][0-9]*\);$/\1/p' src/cleanup.rs)"
if [ -z "$wipe_revision" ]; then
    echo "ct asm evidence: wipe primitive revision is missing" >&2
    exit 1
fi

case "$target_triple" in
    x86_64-*|i686-*)
        wipe_barrier="mfence plus compiler fence"
        result_gate_barrier="lfence plus compiler fence"
        ;;
    aarch64-*)
        wipe_barrier="dsb sy, isb sy, CSDB hint, plus compiler fence"
        result_gate_barrier="isb sy, CSDB hint, plus compiler fence"
        ;;
    arm-*)
        wipe_barrier="dsb sy, isb sy, plus compiler fence"
        result_gate_barrier="isb sy plus compiler fence"
        ;;
    riscv32-*|riscv64*)
        wipe_barrier="fence rw,rw plus compiler fence"
        result_gate_barrier="fence rw,rw plus compiler fence"
        ;;
    wasm32-*)
        wipe_barrier="compiler fence only; downstream JIT outside evidence"
        result_gate_barrier="compiler fence only; downstream JIT outside evidence"
        ;;
    *)
        wipe_barrier="compiler fence only"
        result_gate_barrier="compiler fence only"
        ;;
esac

echo "ct asm evidence: no-default-features release assembly"
if [ -n "$target" ]; then
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/no-default" \
        cargo rustc --locked --release --lib --no-default-features \
        --target "$target" -- --emit=asm
else
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/no-default" \
        cargo rustc --locked --release --lib --no-default-features -- --emit=asm
fi
copy_single_asm "$(release_deps_dir "$audit_root/no-default")" \
    "$output_dir/base64_ng-no-default-features.s"

echo "ct asm evidence: all-features release assembly"
if [ -n "$target" ]; then
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/all-features" \
        cargo rustc --locked --release --lib --all-features \
        --target "$target" -- --emit=asm
else
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/all-features" \
        cargo rustc --locked --release --lib --all-features -- --emit=asm
fi
copy_single_asm "$(release_deps_dir "$audit_root/all-features")" \
    "$output_dir/base64_ng-all-features.s"
require_reviewed_symbols "$output_dir/base64_ng-all-features.s"

echo "ct asm evidence: all-features LTO release assembly"
if [ -n "$target" ]; then
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/all-features-lto" \
    RUSTFLAGS="-C lto=fat -C embed-bitcode=yes" \
        cargo rustc --locked --release --lib --all-features \
        --target "$target" -- --emit=asm
else
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/all-features-lto" \
    RUSTFLAGS="-C lto=fat -C embed-bitcode=yes" \
        cargo rustc --locked --release --lib --all-features -- --emit=asm
fi
copy_single_asm "$(release_deps_dir "$audit_root/all-features-lto")" \
    "$output_dir/base64_ng-all-features-lto.s"
require_reviewed_symbols "$output_dir/base64_ng-all-features-lto.s"
evidence_verify_source "ct asm evidence"

{
    echo "base64-ng constant-time assembly evidence"
    echo
    evidence_write_source_manifest
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "evidence boundary:"
    echo "target=$target_triple"
    echo "target_mode=$target_mode"
    echo "target_cfg=$(rustc --print cfg --target "$target_triple" | tr '\n' ' ')"
    echo "release_profile=optimized"
    echo "feature_modes=no-default-features,all-features"
    echo "lto_flags=-C lto=fat -C embed-bitcode=yes"
    echo "wipe_primitive_revision=$wipe_revision"
    echo "runtime_wipe_generation=operation-report-specific; not inferred from assembly"
    echo "wipe_barrier=$wipe_barrier"
    echo "result_gate_barrier=$result_gate_barrier"
    echo "wipe_scope=logical-range volatile overwrite and selected generated barrier only"
    echo "wipe_excludes=registers,caches,allocator-history,swap,snapshots,compiler-copies"
    echo
    echo "commands:"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/no-default cargo rustc --locked --release --lib --no-default-features [--target $target_triple] -- --emit=asm"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/all-features cargo rustc --locked --release --lib --all-features [--target $target_triple] -- --emit=asm"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/all-features-lto RUSTFLAGS=\"-C lto=fat -C embed-bitcode=yes\" cargo rustc --locked --release --lib --all-features [--target $target_triple] -- --emit=asm"
    echo
    echo "artifacts:"
    evidence_checksum_file "$output_dir/base64_ng-no-default-features.s"
    evidence_checksum_file "$output_dir/base64_ng-all-features.s"
    evidence_checksum_file "$output_dir/base64_ng-all-features-lto.s"
    echo
    echo "review focus:"
    echo "- ct::CtEngine decode entry points"
    echo "- ct_decode_* scalar helper code"
    echo "- ct_decode_alphabet_byte generic alphabet scanner"
    echo "- ct_mask_* arithmetic helpers"
    echo "- absence of secret-indexed lookup tables in ct symbol mapping"
    echo "- absence of secret-byte-class branches in fixed-length ct decode loops"
    echo "- constant_time_eq_public_len equal-length comparison helper"
    echo "- ct_error_gate_barrier remains a non-inlined malformed-input gate boundary"
    echo "- secret_encode_ascii uses arithmetic mapping without secret-indexed alphabet loads"
    echo "- secret_encode_scan performs a fixed 64-entry public-index scan for custom alphabets"
    echo "- v2 secret decode decode_symbol performs a fixed 64-entry public-index scan"
    echo "- secret encoder mapping has no secret-value branches in the reviewed target assembly"
    echo "- wipe_bytes and wipe_barrier remain non-inlined cleanup call boundaries"
    echo "- release and LTO artifacts contain separate wipe, comparison, CT gate, secret decode, and secret encode mapping text symbols"
    echo "- symbol checks accept legacy Rust mangling and Rust 1.97+ v0 mangling"
} >"$manifest"

echo "ct asm evidence: wrote $output_dir"
