#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/neon-asm"
manifest="$output_dir/MANIFEST.txt"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-neon-asm.XXXXXX")"
target="${BASE64_NG_NEON_ASM_TARGET:-aarch64-unknown-linux-gnu}"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
evidence_capture_source "NEON asm evidence"

echo "NEON asm evidence: AArch64 release library assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/build" \
    cargo rustc --locked --target "$target" --release \
    --all-features --lib -- --emit=asm

set -- "$audit_root"/build/"$target"/release/deps/base64_ng-*.s
if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "NEON asm evidence: expected one fresh assembly artifact" >&2
    exit 1
fi
cp "$1" "$output_dir/base64_ng-neon-aarch64.s"
assembly="$output_dir/base64_ng-neon-aarch64.s"

require_pattern() {
    pattern="$1"
    description="$2"
    if ! grep -E -q "$pattern" "$assembly"; then
        echo "NEON asm evidence: missing $description" >&2
        exit 1
    fi
}

python3 scripts/check-neon-validation-codegen.py "$output_dir" assembly
python3 scripts/check-neon-validation-codegen.py "$output_dir" direct-reduction
require_pattern '(tbl[[:space:]]+v[0-9]+\.16b|tbl\.16b[[:space:]]+v[0-9]+)' \
    'NEON byte-table permutation'
require_pattern '(bsl[[:space:]]+v[0-9]+\.16b|bsl\.16b[[:space:]]+v[0-9]+)' \
    'NEON alphabet mapping select'
require_pattern '(st1[[:space:]]+\{ v[0-9]+\.s \}\[2\]|st1\.s[[:space:]]+\{ v[0-9]+ \}\[2\])' \
    'exact final four-byte decode store'
require_pattern '(eor[[:space:]]+v0\.16b, v0\.16b, v0\.16b|eor\.16b[[:space:]]+v0, v0, v0)' \
    'one-per-call NEON register cleanup sequence'

evidence_verify_source "NEON asm evidence"
{
    echo "base64-ng Commit 29 AArch64 NEON assembly evidence"
    echo
    evidence_write_source_manifest
    echo
    rustc -Vv
    echo
    cargo -V
    echo
    echo "command:"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh> cargo rustc --locked --target $target --release --all-features --lib -- --emit=asm"
    echo
    echo "artifact:"
    evidence_checksum_file "$assembly"
    echo
    echo "review focus:"
    echo "- encode uses exact 8+4-byte input reads, table permutation, and vector alphabet mapping"
    echo "- validation-only classifiers pass the lane-mask and reduction-to-bool semantic checker"
    echo "- direct decode reduction inventory accepts uminv or a full-width addp/fmov/cmn/b.ne pattern; direct kernel tests check lane behavior"
    echo "- packing inventory includes tbl and exact 8+4-byte stores; this pattern check is not whole-program dataflow proof"
    echo "- wrapper loops clear AArch64 vector state once after the direct block sequence"
} >"$manifest"

echo "NEON asm evidence: wrote $output_dir"
