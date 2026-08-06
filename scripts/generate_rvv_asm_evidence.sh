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

for tool in as objcopy objdump nm readelf; do
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

cat >"$audit_root/rvv-attributes.s" <<'EOF'
.attribute arch, "rv64gcv"
.text
EOF
"$prefix-as" -o "$audit_root/rvv-attributes.o" "$audit_root/rvv-attributes.s"
"$prefix-objcopy" \
    --dump-section .riscv.attributes="$audit_root/rvv.attributes" \
    "$audit_root/rvv-attributes.o"
cp "$binary" "$audit_root/disassembly-binary"
"$prefix-objcopy" \
    --update-section .riscv.attributes="$audit_root/rvv.attributes" \
    "$audit_root/disassembly-binary"
"$prefix-objcopy" --dump-section .text="$audit_root/original.text" "$binary"
"$prefix-objcopy" \
    --dump-section .text="$audit_root/disassembly.text" \
    "$audit_root/disassembly-binary"
if ! cmp -s "$audit_root/original.text" "$audit_root/disassembly.text"; then
    echo "RVV asm evidence: disassembly metadata copy changed executable text" >&2
    exit 1
fi
original_text_digest="$(evidence_checksum_file "$audit_root/original.text" | awk '{print $1}')"
disassembly_text_digest="$(evidence_checksum_file "$audit_root/disassembly.text" | awk '{print $1}')"

symbols="base64_ng_rvv_encode_standard_quanta base64_ng_rvv_encode_url_safe_quanta base64_ng_rvv_decode_standard_quanta base64_ng_rvv_decode_url_safe_quanta base64_ng_rvv_vlenb base64_ng_rvv_signal_context_round_trip base64_ng_rvv_signal_clobber"
: >"$output_dir/disassembly.txt"
for symbol in $symbols; do
    if ! "$prefix-nm" "$binary" | grep -E -q "[[:space:]][Tt][[:space:]]+$symbol$"; then
        echo "RVV asm evidence: missing leaf symbol $symbol" >&2
        exit 1
    fi
    "$prefix-objdump" \
        -d --disassemble="$symbol" "$audit_root/disassembly-binary" \
        >>"$output_dir/disassembly.txt"
done
"$prefix-readelf" -A "$binary" >"$output_dir/attributes.txt"
{
    echo "original_text_sha256=$original_text_digest"
    echo "disassembly_text_sha256=$disassembly_text_digest"
    echo "disassembly_attribute=rv64gcv"
    echo "text_identity=verified"
} >"$output_dir/disassembly-metadata.txt"

require_pattern() {
    pattern="$1"
    description="$2"
    if ! grep -E -q "$pattern" "$output_dir/disassembly.txt"; then
        echo "RVV asm evidence: missing $description" >&2
        exit 1
    fi
}

require_pattern 'vsetvli[[:space:]]+a3,a2,e8,m1,ta,ma' 'vector-length-agnostic quantum batching'
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
if grep -E -q 'Tag_RISCV_arch:.*_v[0-9]' "$output_dir/attributes.txt"; then
    echo "RVV asm evidence: linked artifact incorrectly requires V globally" >&2
    exit 1
fi

evidence_verify_source "RVV asm evidence"
{
    echo "base64-ng exact-profile RVV assembly evidence"
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
    echo "production_admission=linux-spacemit-x60-only"
    echo "disassembly_copy=text-identical-attribute-only"
    echo "symbols=$symbols"
    echo
    echo "artifacts:"
    evidence_checksum_file "$output_dir/disassembly.txt"
    evidence_checksum_file "$output_dir/attributes.txt"
    evidence_checksum_file "$output_dir/disassembly-metadata.txt"
    echo
    echo "review focus:"
    echo "- leaf ABI with no calls or stack mutation"
    echo "- VLEN-agnostic batched segment loads/stores"
    echo "- Standard and URL-safe arithmetic mapping"
    echo "- v0..v15 cleared at VLMAX before every return"
    echo "- vector instructions remain leaf-local; the ELF does not globally require V"
    echo "- readable disassembly comes from a text-identical copy with disassembly-only V metadata"
    echo "- normal production dispatch requires the exact Linux SpacemiT X60 profile"
} >"$manifest"

echo "RVV asm evidence: wrote $output_dir"
