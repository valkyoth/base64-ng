#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/rvv-asm"
manifest="$output_dir/MANIFEST.txt"
target="riscv64gc-unknown-linux-gnu"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-rvv-asm.XXXXXX")"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
evidence_capture_source "RVV asm evidence"

if command -v riscv64-suse-linux-gcc >/dev/null 2>&1; then
    linker="riscv64-suse-linux-gcc"
    prefix="riscv64-suse-linux"
elif command -v riscv64-linux-gnu-gcc >/dev/null 2>&1; then
    linker="riscv64-linux-gnu-gcc"
    prefix="riscv64-linux-gnu"
else
    echo "RVV asm evidence: no supported riscv64 cross toolchain found" >&2
    exit 1
fi

for tool in objdump nm readelf; do
    if ! command -v "$prefix-$tool" >/dev/null 2>&1; then
        echo "RVV asm evidence: missing $prefix-$tool" >&2
        exit 1
    fi
done

echo "RVV asm evidence: release candidate binary"
env \
    RUSTFLAGS="${RUSTFLAGS:-} --cfg base64_ng_rvv_candidate" \
    CARGO_INCREMENTAL=0 \
    CARGO_TARGET_DIR="$audit_root/build" \
    CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER="$linker" \
    cargo test --locked --target "$target" --release --all-features --lib --no-run

set -- "$audit_root"/build/"$target"/release/deps/base64_ng-*
binary=""
for candidate in "$@"; do
    if [ -f "$candidate" ] && [ -x "$candidate" ]; then
        if [ -n "$binary" ]; then
            echo "RVV asm evidence: expected one fresh test binary" >&2
            exit 1
        fi
        binary="$candidate"
    fi
done
if [ -z "$binary" ]; then
    echo "RVV asm evidence: fresh test binary not found" >&2
    exit 1
fi

symbols="base64_ng_rvv_encode_standard_12 base64_ng_rvv_encode_url_safe_12 base64_ng_rvv_decode_standard_16 base64_ng_rvv_decode_url_safe_16 base64_ng_rvv_vlenb base64_ng_rvv_signal_context_round_trip base64_ng_rvv_signal_clobber"
: >"$output_dir/disassembly.txt"
for symbol in $symbols; do
    if ! "$prefix-nm" "$binary" | grep -E -q "[[:space:]][Tt][[:space:]]+$symbol$"; then
        echo "RVV asm evidence: missing leaf symbol $symbol" >&2
        exit 1
    fi
    "$prefix-objdump" -d --disassemble="$symbol" "$binary" >>"$output_dir/disassembly.txt"
done
"$prefix-readelf" -A "$binary" >"$output_dir/attributes.txt"

require_pattern() {
    pattern="$1"
    description="$2"
    if ! grep -E -q "$pattern" "$output_dir/disassembly.txt"; then
        echo "RVV asm evidence: missing $description" >&2
        exit 1
    fi
}

require_pattern 'vsetivli[[:space:]]+zero,4,e8,m1,ta,ma' 'vector-length-agnostic four-lane setup'
require_pattern 'vlseg3e8\.v' 'three-segment encode load'
require_pattern 'vsseg4e8\.v' 'four-segment encode store'
require_pattern 'vlseg4e8\.v' 'four-segment decode load'
require_pattern 'vsseg3e8\.v' 'three-segment decode store'
require_pattern 'vmseq\.vx' 'alphabet-special classification masks'
require_pattern 'vmv\.v\.i[[:space:]]+v15,0' 'full used-register cleanup tail'
require_pattern 'amoswap\.w' 'syscall-free native signal-test synchronization'

if grep -E -q '[[:space:]]jal[[:space:]]' "$output_dir/disassembly.txt"; then
    echo "RVV asm evidence: leaf candidate contains an unexpected call" >&2
    exit 1
fi
if ! grep -E -q 'v1p0|_v' "$output_dir/attributes.txt"; then
    echo "RVV asm evidence: linked candidate does not advertise the V extension" >&2
    exit 1
fi

evidence_verify_source "RVV asm evidence"
{
    echo "base64-ng Commit 32 non-admitted RVV assembly evidence"
    echo
    evidence_write_source_manifest
    echo
    rustc -Vv
    echo
    cargo -V
    echo
    echo "target=$target"
    echo "linker=$linker"
    echo "candidate_cfg=base64_ng_rvv_candidate"
    echo "production_admission=false"
    echo "symbols=$symbols"
    echo
    echo "artifacts:"
    evidence_checksum_file "$output_dir/disassembly.txt"
    evidence_checksum_file "$output_dir/attributes.txt"
    echo
    echo "review focus:"
    echo "- fixed leaf ABI with no calls or stack mutation"
    echo "- VLEN-agnostic four-lane segment loads/stores"
    echo "- Standard and URL-safe arithmetic mapping"
    echo "- v0..v15 cleared at VLMAX before every return"
    echo "- candidate remains unavailable to normal production dispatch"
} >"$manifest"

echo "RVV asm evidence: wrote $output_dir"
