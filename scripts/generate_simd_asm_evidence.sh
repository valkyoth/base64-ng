#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/simd-asm"
manifest="$output_dir/MANIFEST.txt"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-simd-asm.XXXXXX")"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
evidence_capture_source "simd asm evidence"

copy_single_asm() {
    target_dir="$1"
    output_file="$2"
    set -- "$target_dir"/*/release/deps/base64_ng-*.s "$target_dir"/release/deps/base64_ng-*.s
    found=""
    count=0
    for candidate in "$@"; do
        if [ -f "$candidate" ]; then
            found="$candidate"
            count=$((count + 1))
        fi
    done

    if [ "$count" -ne 1 ]; then
        echo "simd asm evidence: expected exactly one fresh assembly file under $target_dir" >&2
        exit 1
    fi

    cp "$found" "$output_file"
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
        evidence_verify_source "simd asm evidence"
        {
            echo "base64-ng SIMD assembly evidence"
            echo
            evidence_write_source_manifest
            echo
            echo "skipped: host $host_triple is not an x86/x86_64 target"
        } >"$manifest"
        echo "simd asm evidence: skipped non-x86 host $host_triple"
        exit 0
        ;;
esac

echo "simd asm evidence: SSSE3/SSE4.1 release test assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/ssse3-sse41" \
RUSTFLAGS="-C target-feature=+ssse3,+sse4.1" \
    cargo rustc --locked --release --all-features --lib -- --emit=asm --test
copy_single_asm "$audit_root/ssse3-sse41" "$output_dir/base64_ng-ssse3-sse41-test.s"
require_pattern "$output_dir/base64_ng-ssse3-sse41-test.s" "vpshufb" "SSSE3 byte-shuffle instruction"
require_pattern "$output_dir/base64_ng-ssse3-sse41-test.s" "vpmaddubsw" "SSSE3 strict-decode byte packing"
require_pattern "$output_dir/base64_ng-ssse3-sse41-test.s" "vpmaddwd" "SSSE3 strict-decode word packing"
require_pattern "$output_dir/base64_ng-ssse3-sse41-test.s" "xmm" "XMM register use"

echo "simd asm evidence: AVX2 release test assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/avx2" \
RUSTFLAGS="-C target-feature=+avx2" \
    cargo rustc --locked --release --all-features --lib -- --emit=asm --test
copy_single_asm "$audit_root/avx2" "$output_dir/base64_ng-avx2-test.s"
require_pattern "$output_dir/base64_ng-avx2-test.s" "vpshufb" "AVX2 byte-shuffle instruction"
require_pattern "$output_dir/base64_ng-avx2-test.s" "vpmaddubsw" "AVX2 strict-decode byte packing"
require_pattern "$output_dir/base64_ng-avx2-test.s" "vpmaddwd" "AVX2 strict-decode word packing"
require_pattern "$output_dir/base64_ng-avx2-test.s" "ymm" "YMM register use"
require_pattern "$output_dir/base64_ng-avx2-test.s" "vzeroupper" "AVX upper-state cleanup"

echo "simd asm evidence: AVX-512 VBMI release test assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/avx512-vbmi" \
RUSTFLAGS="-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi" \
    cargo rustc --locked --release --all-features --lib -- --emit=asm --test
copy_single_asm "$audit_root/avx512-vbmi" "$output_dir/base64_ng-avx512-vbmi-test.s"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vpermb" "AVX-512 VBMI byte-permute instruction"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vpmaddubsw" "AVX-512 strict-decode byte packing"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vpmaddwd" "AVX-512 strict-decode word packing"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "zmm" "ZMM register use"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vpxord[[:space:]]+%zmm0" "ZMM cleanup sequence"
require_pattern "$output_dir/base64_ng-avx512-vbmi-test.s" "vzeroupper" "AVX upper-state cleanup"

neon_status="skipped-target-not-installed"
neon_decode_status="skipped-target-not-installed"
if rustup target list --installed 2>/dev/null | grep -F -x -q "aarch64-unknown-linux-gnu"; then
    host_triple="$(rustc -vV | sed -n 's/^host: //p')"
    if printf '%s\n' "$host_triple" | grep -q '^aarch64-'; then
        echo "simd asm evidence: AArch64 NEON release test assembly"
        CARGO_INCREMENTAL=0 \
        CARGO_TARGET_DIR="$audit_root/neon-aarch64" \
            cargo rustc --locked --target aarch64-unknown-linux-gnu --release --all-features --lib -- --emit=asm --test
        neon_decode_status="generated"
    else
        echo "simd asm evidence: AArch64 NEON release library assembly"
        CARGO_INCREMENTAL=0 \
        CARGO_TARGET_DIR="$audit_root/neon-aarch64" \
            cargo rustc --locked --target aarch64-unknown-linux-gnu --release --all-features --lib -- --emit=asm
        neon_decode_status="skipped-cross-host-test-link"
    fi
    copy_single_asm "$audit_root/neon-aarch64" "$output_dir/base64_ng-neon-aarch64-test.s"
    require_pattern "$output_dir/base64_ng-neon-aarch64-test.s" "tbl" "AArch64 NEON table lookup instruction"
    require_pattern "$output_dir/base64_ng-neon-aarch64-test.s" "bsl" "AArch64 NEON bit-select instruction"
    require_pattern "$output_dir/base64_ng-neon-aarch64-test.s" "eor[[:space:]]+v0\\.16b" "NEON register cleanup sequence"
    neon_status="generated"
fi
evidence_verify_source "simd asm evidence"

{
    echo "base64-ng SIMD assembly evidence"
    echo
    evidence_write_source_manifest
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
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/ssse3-sse41 RUSTFLAGS=\"-C target-feature=+ssse3,+sse4.1\" cargo rustc --locked --release --all-features --lib -- --emit=asm --test"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/avx2 RUSTFLAGS=\"-C target-feature=+avx2\" cargo rustc --locked --release --all-features --lib -- --emit=asm --test"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/avx512-vbmi RUSTFLAGS=\"-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi\" cargo rustc --locked --release --all-features --lib -- --emit=asm --test"
    if [ "$neon_status" = "generated" ]; then
        if [ "$neon_decode_status" = "generated" ]; then
            echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/neon-aarch64 cargo rustc --locked --target aarch64-unknown-linux-gnu --release --all-features --lib -- --emit=asm --test"
        else
            echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh>/neon-aarch64 cargo rustc --locked --target aarch64-unknown-linux-gnu --release --all-features --lib -- --emit=asm"
        fi
    else
        echo "AArch64 NEON assembly skipped: aarch64-unknown-linux-gnu target is not installed"
    fi
    echo
    echo "artifacts:"
    evidence_checksum_file "$output_dir/base64_ng-ssse3-sse41-test.s"
    evidence_checksum_file "$output_dir/base64_ng-avx2-test.s"
    evidence_checksum_file "$output_dir/base64_ng-avx512-vbmi-test.s"
    if [ "$neon_status" = "generated" ]; then
        evidence_checksum_file "$output_dir/base64_ng-neon-aarch64-test.s"
    fi
    echo
    echo "review focus:"
    echo "- SSSE3/SSE4.1 admitted encode path contains exact-width input reads, byte shuffle, XMM operations, and no per-block cleanup"
    echo "- SSSE3/SSE4.1 admitted strict decode path contains direct ASCII classification, 6-bit mapping, multiply-add packing, exact 12-byte stores, XMM operations, and one cleanup at the block-loop boundary"
    echo "- AVX2 admitted encode path contains exact-width input reads, byte shuffle, YMM operations, and one-per-call vzeroupper"
    echo "- AVX2 admitted strict decode path contains direct ASCII classification, 6-bit mapping, exact per-lane 12-byte stores, YMM operations, and one cleanup at the block-loop boundary"
    echo "- AVX-512 admitted encode path contains an exact 48-lane masked load, direct VBMI expansion and alphabet permutes, ZMM operations, one-per-call ZMM cleanup, and vzeroupper"
    echo "- AVX-512 VBMI admitted strict decode path contains byte shuffle, multiply-add packing, VBMI lane compaction, ZMM operations, ZMM cleanup, and vzeroupper"
    if [ "$neon_status" = "generated" ]; then
        echo "- NEON admitted encode path contains AArch64 table lookup, bit-select mapping, and NEON cleanup"
        if [ "$neon_decode_status" = "generated" ]; then
            echo "- NEON admitted strict decode path contains AArch64 table compaction, vector shift/mask packing, and NEON cleanup"
        else
            echo "- NEON admitted strict decode test-harness assembly evidence requires an AArch64 host; this cross-host run recorded library assembly and compile evidence only"
        fi
    else
        echo "- NEON admitted encode and strict decode assembly evidence was skipped because the AArch64 target is not installed"
    fi
    echo "- AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and NEON encode are admitted for std x86/x86_64 or little-endian std aarch64 Standard and URL-safe alphabets"
} >"$manifest"

echo "simd asm evidence: wrote $output_dir"
