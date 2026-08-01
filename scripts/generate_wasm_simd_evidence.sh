#!/usr/bin/env sh
set -eu

wasm_target="${1:-wasm32-unknown-unknown}"
output_dir="target/release-evidence/wasm-simd"
manifest="$output_dir/MANIFEST.txt"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-wasm-simd.XXXXXX")"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
evidence_capture_source "wasm simd evidence"

require_pattern() {
    file="$1"
    pattern="$2"
    description="$3"

    if ! grep -E -q "$pattern" "$file"; then
        echo "wasm simd evidence: missing $description in $file" >&2
        exit 1
    fi
}

if ! rustup target list --installed 2>/dev/null | grep -F -x -q "$wasm_target"; then
    evidence_verify_source "wasm simd evidence"
    {
        echo "base64-ng wasm simd128 codegen evidence"
        echo
        evidence_write_source_manifest
        echo
        echo "skipped: target $wasm_target is not installed"
    } >"$manifest"
    echo "wasm simd evidence: skipping $wasm_target; Rust target is not installed"
    exit 0
fi

echo "wasm simd evidence: release test-harness LLVM IR for $wasm_target"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/target" \
RUSTFLAGS='-C target-feature=+simd128' \
    cargo rustc --locked --target "$wasm_target" --release \
        --features simd,allow-wasm32-best-effort-wipe \
        --lib -- --emit=llvm-ir --test

set -- "$audit_root"/target/*/release/deps/base64_ng-*.ll
if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "wasm simd evidence: expected exactly one fresh LLVM IR file" >&2
    exit 1
fi

artifact="$output_dir/base64_ng-wasm-simd128-test.ll"
cp "$1" "$artifact"
test -s "$artifact"

require_pattern "$artifact" 'target triple = "wasm32-unknown-unknown"' "wasm32 target triple"
require_pattern "$artifact" '"target-features"="\+simd128"' "simd128 target feature"
require_pattern "$artifact" "encode_12_bytes_wasm_simd128" "anchored wasm prototype symbol"
require_pattern "$artifact" "shufflevector" "vector shuffle operation"
require_pattern "$artifact" "<16 x i8>" "128-bit byte-vector operation"
require_pattern "$artifact" "llvm\\.wasm\\.bitselect\\.v16i8" "wasm bitselect intrinsic"
evidence_verify_source "wasm simd evidence"

{
    echo "base64-ng wasm simd128 codegen evidence"
    echo
    evidence_write_source_manifest
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "cargo:"
    cargo -V
    echo
    echo "command:"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/target RUSTFLAGS='-C target-feature=+simd128' cargo rustc --locked --target $wasm_target --release --features simd,allow-wasm32-best-effort-wipe --lib -- --emit=llvm-ir --test"
    echo
    echo "artifacts:"
    evidence_checksum_file "$artifact"
    echo
    echo "review focus:"
    echo "- wasm simd128 release codegen evidence for the admitted narrow runtime profile"
    echo "- test-harness LLVM IR contains fixed-block encode and decode vector code"
    echo "- IR contains simd128 target features, vector shuffle, 128-bit byte vectors, and wasm bitselect"
    echo "- this evidence does not execute wasm and does not attest any runtime/JIT timing or cleanup behavior; runtime dispatch is checked separately by scripts/check_wasm_runtime_dispatch.sh"
} >"$manifest"

echo "wasm simd evidence: wrote $output_dir"
