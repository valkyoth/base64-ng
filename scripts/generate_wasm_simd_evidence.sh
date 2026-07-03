#!/usr/bin/env sh
set -eu

wasm_target="${1:-wasm32-unknown-unknown}"
output_dir="target/release-evidence/wasm-simd"
manifest="$output_dir/MANIFEST.txt"
mkdir -p "$output_dir"

checksum_file() {
    file="$1"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file"
    else
        cksum "$file"
    fi
}

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
    {
        echo "base64-ng wasm simd128 codegen evidence"
        echo
        echo "skipped: target $wasm_target is not installed"
    } >"$manifest"
    echo "wasm simd evidence: skipping $wasm_target; Rust target is not installed"
    exit 0
fi

echo "wasm simd evidence: release test-harness LLVM IR for $wasm_target"
CARGO_TARGET_DIR="target/wasm-simd-evidence" \
RUSTFLAGS='-C target-feature=+simd128' \
    cargo rustc --target "$wasm_target" --release \
        --features simd,allow-wasm32-best-effort-wipe \
        --lib -- --emit=llvm-ir --test

ir_file="$(
    find target/wasm-simd-evidence -path '*/release/deps/base64_ng-*.ll' -type f \
        | sort \
        | sed -n '1p'
)"

if [ -z "$ir_file" ]; then
    echo "wasm simd evidence: no LLVM IR file found" >&2
    exit 1
fi

artifact="$output_dir/base64_ng-wasm-simd128-test.ll"
cp "$ir_file" "$artifact"
test -s "$artifact"

require_pattern "$artifact" 'target triple = "wasm32-unknown-unknown"' "wasm32 target triple"
require_pattern "$artifact" '"target-features"="\+simd128"' "simd128 target feature"
require_pattern "$artifact" "encode_12_bytes_wasm_simd128" "anchored wasm prototype symbol"
require_pattern "$artifact" "shufflevector" "vector shuffle operation"
require_pattern "$artifact" "<16 x i8>" "128-bit byte-vector operation"
require_pattern "$artifact" "llvm\\.wasm\\.bitselect\\.v16i8" "wasm bitselect intrinsic"

{
    echo "base64-ng wasm simd128 codegen evidence"
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "cargo:"
    cargo -V
    echo
    echo "command:"
    echo "CARGO_TARGET_DIR=target/wasm-simd-evidence RUSTFLAGS='-C target-feature=+simd128' cargo rustc --target $wasm_target --release --features simd,allow-wasm32-best-effort-wipe --lib -- --emit=llvm-ir --test"
    echo
    echo "artifacts:"
    checksum_file "$artifact"
    echo
    echo "review focus:"
    echo "- wasm simd128 remains non-dispatchable runtime evidence"
    echo "- test-harness LLVM IR contains the inactive fixed-block encode prototype"
    echo "- IR contains simd128 target features, vector shuffle, 128-bit byte vectors, and wasm bitselect"
    echo "- this evidence does not execute wasm and does not attest any runtime/JIT timing or cleanup behavior"
} >"$manifest"

echo "wasm simd evidence: wrote $output_dir"
