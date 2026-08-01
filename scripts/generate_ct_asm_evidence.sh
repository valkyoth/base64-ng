#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/asm"
manifest="$output_dir/MANIFEST.txt"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-ct-asm.XXXXXX")"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
evidence_capture_source "ct asm evidence"

copy_single_asm() {
    target_dir="$1"
    output_file="$2"
    set -- "$target_dir"/release/deps/base64_ng-*.s

    if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
        echo "ct asm evidence: expected exactly one fresh assembly file under $target_dir" >&2
        exit 1
    fi

    cp "$1" "$output_file"
    test -s "$output_file"
}

require_lto_symbol() {
    symbol_len="$1"
    symbol_name="$2"
    legacy_pattern="^[[:space:]]*\\.section[[:space:]]+\\.text\\._ZN9base64_ng.*${symbol_len}${symbol_name}17h"
    v0_pattern="^[[:space:]]*\\.section[[:space:]]+\\.text\\._R.*9base64_ng.*${symbol_len}${symbol_name},"

    if grep -E -q "$legacy_pattern" "$output_dir/base64_ng-all-features-lto.s"; then
        return
    fi

    if grep -E -q "$v0_pattern" "$output_dir/base64_ng-all-features-lto.s"; then
        return
    fi

    echo "ct asm evidence: missing non-inlined ${symbol_name} symbol in LTO assembly" >&2
    exit 1
}

echo "ct asm evidence: no-default-features release assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/no-default" \
    cargo rustc --locked --release --lib --no-default-features -- --emit=asm
copy_single_asm "$audit_root/no-default" "$output_dir/base64_ng-no-default-features.s"

echo "ct asm evidence: all-features release assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/all-features" \
    cargo rustc --locked --release --lib --all-features -- --emit=asm
copy_single_asm "$audit_root/all-features" "$output_dir/base64_ng-all-features.s"

echo "ct asm evidence: all-features LTO release assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/all-features-lto" \
RUSTFLAGS="-C lto=fat -C embed-bitcode=yes" \
    cargo rustc --locked --release --lib --all-features -- --emit=asm
copy_single_asm "$audit_root/all-features-lto" "$output_dir/base64_ng-all-features-lto.s"
require_lto_symbol "10" "wipe_bytes"
require_lto_symbol "12" "wipe_barrier"
require_lto_symbol "27" "constant_time_eq_public_len"
require_lto_symbol "21" "ct_error_gate_barrier"
require_lto_symbol "19" "secret_encode_ascii"
require_lto_symbol "18" "secret_encode_scan"
evidence_verify_source "ct asm evidence"

{
    echo "base64-ng constant-time assembly evidence"
    echo
    evidence_write_source_manifest
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "commands:"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/no-default cargo rustc --locked --release --lib --no-default-features -- --emit=asm"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/all-features cargo rustc --locked --release --lib --all-features -- --emit=asm"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/all-features-lto RUSTFLAGS=\"-C lto=fat -C embed-bitcode=yes\" cargo rustc --locked --release --lib --all-features -- --emit=asm"
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
    echo "- secret encoder mapping has no secret-value branches in the reviewed target assembly"
    echo "- wipe_bytes and wipe_barrier remain non-inlined cleanup call boundaries"
    echo "- LTO artifact contains separate wipe, comparison, CT gate, and secret encoder mapping text symbols"
    echo "- symbol checks accept legacy Rust mangling and Rust 1.97+ v0 mangling"
} >"$manifest"

echo "ct asm evidence: wrote $output_dir"
