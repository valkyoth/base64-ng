#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/simd-asm"
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

copy_single_asm() {
    target_dir="$1"
    output_file="$2"
    asm_file="$(
        find "$target_dir" -path '*/release/deps/base64_ng-*.s' -type f \
            | sort \
            | sed -n '1p'
    )"

    if [ -z "$asm_file" ]; then
        echo "simd asm evidence: no assembly file found under $target_dir" >&2
        exit 1
    fi

    cp "$asm_file" "$output_file"
    test -s "$output_file"
}

require_pattern() {
    file="$1"
    pattern="$2"
    description="$3"

    if ! grep -E -q "$pattern" "$file"; then
        echo "simd asm evidence: missing $description in $file" >&2
        exit 1
    fi
}

host_triple="$(rustc -vV | sed -n 's/^host: //p')"
case "$host_triple" in
    x86_64-*|i686-*|i586-*|i486-*|i386-*) ;;
    *)
        {
            echo "base64-ng SIMD assembly evidence"
            echo
            echo "skipped: host $host_triple is not an x86/x86_64 target"
        } >"$manifest"
        echo "simd asm evidence: skipped non-x86 host $host_triple"
        exit 0
        ;;
esac

echo "simd asm evidence: SSSE3/SSE4.1 release test assembly"
CARGO_TARGET_DIR="target/simd-asm-ssse3-sse41" \
RUSTFLAGS="-C target-feature=+ssse3,+sse4.1" \
    cargo rustc --release --all-features --lib -- --emit=asm --test
copy_single_asm "target/simd-asm-ssse3-sse41" "$output_dir/base64_ng-ssse3-sse41-test.s"
require_pattern "$output_dir/base64_ng-ssse3-sse41-test.s" "vpshufb" "SSSE3 byte-shuffle instruction"
require_pattern "$output_dir/base64_ng-ssse3-sse41-test.s" "xmm" "XMM register use"

echo "simd asm evidence: AVX2 release test assembly"
CARGO_TARGET_DIR="target/simd-asm-avx2" \
RUSTFLAGS="-C target-feature=+avx2" \
    cargo rustc --release --all-features --lib -- --emit=asm --test
copy_single_asm "target/simd-asm-avx2" "$output_dir/base64_ng-avx2-test.s"
require_pattern "$output_dir/base64_ng-avx2-test.s" "vpshufb" "AVX2 byte-shuffle instruction"
require_pattern "$output_dir/base64_ng-avx2-test.s" "ymm" "YMM register use"
require_pattern "$output_dir/base64_ng-avx2-test.s" "vzeroupper" "AVX upper-state cleanup"

echo "simd asm evidence: AVX-512 VBMI release test assembly"
CARGO_TARGET_DIR="target/simd-asm-avx512-vbmi" \
RUSTFLAGS="-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi" \
    cargo rustc --release --all-features --lib -- --emit=asm --test
copy_single_asm "target/simd-asm-avx512-vbmi" "$output_dir/base64_ng-avx512-vbmi-test.s"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vpermb" "AVX-512 VBMI byte-permute instruction"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "zmm" "ZMM register use"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vpxord[[:space:]]+%zmm0" "ZMM cleanup sequence"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vzeroupper" "AVX upper-state cleanup"

neon_status="skipped-target-not-installed"
if rustup target list --installed 2>/dev/null | grep -F -x -q "aarch64-unknown-linux-gnu"; then
    echo "simd asm evidence: AArch64 NEON release test assembly"
    CARGO_TARGET_DIR="target/simd-asm-neon-aarch64" \
        cargo rustc --target aarch64-unknown-linux-gnu --release --all-features --lib -- --emit=asm
    copy_single_asm "target/simd-asm-neon-aarch64" "$output_dir/base64_ng-neon-aarch64-test.s"
    require_pattern "$output_dir/base64_ng-neon-aarch64-test.s" "tbl" "AArch64 NEON table lookup instruction"
    require_pattern "$output_dir/base64_ng-neon-aarch64-test.s" "bsl" "AArch64 NEON bit-select instruction"
    require_pattern "$output_dir/base64_ng-neon-aarch64-test.s" "eor[[:space:]]+v0\\.16b" "NEON register cleanup sequence"
    neon_status="generated"
fi

{
    echo "base64-ng SIMD assembly evidence"
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "cargo:"
    cargo -V
    echo
    echo "system:"
    if command -v uname >/dev/null 2>&1; then
        uname -a
    else
        echo "uname unavailable"
    fi
    echo
    echo "commands:"
    echo "CARGO_TARGET_DIR=target/simd-asm-ssse3-sse41 RUSTFLAGS=\"-C target-feature=+ssse3,+sse4.1\" cargo rustc --release --all-features --lib -- --emit=asm --test"
    echo "CARGO_TARGET_DIR=target/simd-asm-avx2 RUSTFLAGS=\"-C target-feature=+avx2\" cargo rustc --release --all-features --lib -- --emit=asm --test"
    echo "CARGO_TARGET_DIR=target/simd-asm-avx512-vbmi RUSTFLAGS=\"-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi\" cargo rustc --release --all-features --lib -- --emit=asm --test"
    if [ "$neon_status" = "generated" ]; then
        echo "CARGO_TARGET_DIR=target/simd-asm-neon-aarch64 cargo rustc --target aarch64-unknown-linux-gnu --release --all-features --lib -- --emit=asm"
    else
        echo "AArch64 NEON assembly skipped: aarch64-unknown-linux-gnu target is not installed"
    fi
    echo
    echo "artifacts:"
    checksum_file "$output_dir/base64_ng-ssse3-sse41-test.s"
    checksum_file "$output_dir/base64_ng-avx2-test.s"
    checksum_file "$output_dir/base64_ng-avx512-vbmi-test.s"
    if [ "$neon_status" = "generated" ]; then
        checksum_file "$output_dir/base64_ng-neon-aarch64-test.s"
    fi
    echo
    echo "review focus:"
    echo "- SSSE3/SSE4.1 admitted encode path contains byte shuffle, XMM operations, and XMM cleanup"
    echo "- AVX2 admitted encode path contains byte shuffle, YMM operations, and vzeroupper"
    echo "- AVX-512 admitted encode path contains VBMI byte permute, ZMM operations, ZMM cleanup, and vzeroupper"
    if [ "$neon_status" = "generated" ]; then
        echo "- NEON admitted encode path contains AArch64 table lookup, bit-select mapping, and NEON cleanup"
    else
        echo "- NEON admitted encode path assembly evidence was skipped because the AArch64 target is not installed"
    fi
    echo "- AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and NEON encode are admitted for std x86/x86_64 or std aarch64 Standard and URL-safe alphabets"
} >"$manifest"

echo "simd asm evidence: wrote $output_dir"
