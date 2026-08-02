#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/sve-asm"
manifest="$output_dir/MANIFEST.txt"
target="aarch64-unknown-linux-musl"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-sve-asm.XXXXXX")"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
evidence_capture_source "SVE asm evidence"

host="$(rustc -vV | sed -n 's/^host: //p')"
linker="$(rustc --print sysroot)/lib/rustlib/$host/bin/rust-lld"
if [ ! -x "$linker" ]; then
    echo "SVE asm evidence: rust-lld is missing from the active toolchain" >&2
    exit 1
fi

binutils_prefix=""
for candidate_prefix in aarch64-linux-gnu aarch64-suse-linux; do
    if command -v "$candidate_prefix-objdump" >/dev/null 2>&1 \
        && command -v "$candidate_prefix-nm" >/dev/null 2>&1 \
        && command -v "$candidate_prefix-readelf" >/dev/null 2>&1; then
        binutils_prefix="$candidate_prefix-"
        break
    fi
done
if [ -z "$binutils_prefix" ]; then
    case "$host" in
        aarch64-*)
            for tool in objdump nm readelf; do
                if ! command -v "$tool" >/dev/null 2>&1; then
                    echo "SVE asm evidence: missing native AArch64 $tool" >&2
                    exit 1
                fi
            done
            ;;
        *)
            echo "SVE asm evidence: missing AArch64 cross-binutils" >&2
            echo "SVE asm evidence: install binutils-aarch64-linux-gnu or cross-aarch64-binutils" >&2
            exit 1
            ;;
    esac
fi
objdump="${binutils_prefix}objdump"
nm="${binutils_prefix}nm"
readelf="${binutils_prefix}readelf"

echo "SVE asm evidence: release candidate binary"
env \
    RUSTFLAGS="${RUSTFLAGS:-} --cfg base64_ng_sve_candidate" \
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/build" \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$linker" \
    cargo test --locked --target "$target" --release --all-features --lib --no-run

set -- "$audit_root"/build/"$target"/release/deps/base64_ng-*
binary=""
for candidate in "$@"; do
    if [ -f "$candidate" ] && [ -x "$candidate" ]; then
        if [ -n "$binary" ]; then
            echo "SVE asm evidence: expected one fresh test binary" >&2
            exit 1
        fi
        binary="$candidate"
    fi
done
if [ -z "$binary" ]; then
    echo "SVE asm evidence: fresh test binary not found" >&2
    exit 1
fi

symbols="base64_ng_sve_encode_standard_12 base64_ng_sve_encode_url_safe_12 base64_ng_sve_decode_standard_16 base64_ng_sve_decode_url_safe_16 base64_ng_sve_vector_length"
: >"$output_dir/disassembly.txt"
for symbol in $symbols; do
    if ! "$nm" "$binary" | grep -E -q "[[:space:]][Tt][[:space:]]+$symbol$"; then
        echo "SVE asm evidence: missing leaf symbol $symbol" >&2
        exit 1
    fi
    "$objdump" -d --disassemble="$symbol" "$binary" >>"$output_dir/disassembly.txt"
done
"$readelf" -h -n "$binary" >"$output_dir/elf.txt"

require_pattern() {
    pattern="$1"
    description="$2"
    if ! grep -E -q "$pattern" "$output_dir/disassembly.txt"; then
        echo "SVE asm evidence: missing $description" >&2
        exit 1
    fi
}

require_pattern 'ptrue[[:space:]]+p0\.b.*vl4' 'vector-length-independent four-lane predicate'
require_pattern 'ld3b' 'three-vector encode load'
require_pattern 'st4b' 'four-vector encode store'
require_pattern 'ld4b' 'four-vector decode load'
require_pattern 'st3b' 'three-vector decode store'
require_pattern 'cmphs|cmpeq' 'alphabet classification predicates'
require_pattern 'mov[[:space:]]+z[0-7]\.b,[[:space:]]+p1/m' 'predicate-based alphabet mapping'
require_pattern 'mov[[:space:]]+z7\.b,[[:space:]]+#0' 'used-vector-register cleanup tail'
require_pattern 'pfalse[[:space:]]+p1\.b' 'used-predicate-register cleanup tail'
require_pattern 'cntb[[:space:]]+x0' 'runtime vector-length instruction'

if grep -E -q '[[:space:]]bl[[:space:]]' "$output_dir/disassembly.txt"; then
    echo "SVE asm evidence: leaf candidate contains an unexpected call" >&2
    exit 1
fi
if grep -E -q '[[:space:]]sp([,[:space:]]|$)' "$output_dir/disassembly.txt"; then
    echo "SVE asm evidence: leaf candidate unexpectedly mutates or addresses the stack" >&2
    exit 1
fi

evidence_verify_source "SVE asm evidence"
{
    echo "base64-ng Commit 33 non-admitted SVE assembly evidence"
    echo
    evidence_write_source_manifest
    echo
    rustc -Vv
    echo
    cargo -V
    echo
    echo "target=$target"
    echo "linker=$linker"
    echo "objdump=$objdump"
    echo "nm=$nm"
    echo "readelf=$readelf"
    echo "candidate_cfg=base64_ng_sve_candidate"
    echo "production_admission=false"
    echo "symbols=$symbols"
    echo
    echo "artifacts:"
    evidence_checksum_file "$output_dir/disassembly.txt"
    evidence_checksum_file "$output_dir/elf.txt"
    echo
    echo "review focus:"
    echo "- fixed base PCS leaf ABI with no calls or stack mutation"
    echo "- vector-length-independent four-lane structured loads/stores"
    echo "- Standard and URL-safe predicate-based mapping"
    echo "- caller-saved z0..z7 and p0..p1 cleared before every return"
    echo "- candidate remains unavailable to normal production dispatch"
} >"$manifest"

echo "SVE asm evidence: wrote $output_dir"
