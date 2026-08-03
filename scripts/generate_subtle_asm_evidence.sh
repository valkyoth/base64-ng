#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/subtle-asm"
output_file="$output_dir/base64_ng_subtle-release.s"
manifest="$output_dir/MANIFEST.txt"
audit_root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-subtle-asm.XXXXXX")"
trap 'rm -rf "$audit_root"' EXIT INT TERM
mkdir -p "$output_dir"

. scripts/evidence-source.sh
. scripts/ct-asm-symbols.sh
evidence_capture_source "subtle asm evidence"

echo "subtle asm evidence: no-default-features release assembly"
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR="$audit_root/build" \
    cargo rustc --locked --release -p base64-ng-subtle \
        --no-default-features --lib -- --emit=asm

set -- "$audit_root"/build/release/deps/base64_ng_subtle-*.s
if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "subtle asm evidence: expected exactly one fresh assembly file" >&2
    exit 1
fi
cp "$1" "$output_file"
test -s "$output_file"

if ! ct_asm_symbol_is_defined "$output_file" "23" "subtle_ct_eq_public_len"; then
    echo "subtle asm evidence: missing reviewed public-length equality symbol" >&2
    exit 1
fi

if grep -E -q '(^|[^[:alnum:]_])(memcmp|bcmp)([^[:alnum:]_]|$)' "$output_file"; then
    echo "subtle asm evidence: equality assembly contains an early-exit compare call" >&2
    exit 1
fi

evidence_verify_source "subtle asm evidence"

{
    echo "base64-ng-subtle optimized equality evidence"
    echo
    evidence_write_source_manifest
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "command:"
    echo "CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<fresh> cargo rustc --locked --release -p base64-ng-subtle --no-default-features --lib -- --emit=asm"
    echo
    echo "artifact:"
    evidence_checksum_file "$output_file"
    echo
    echo "review focus:"
    echo "- subtle_ct_eq_public_len remains a separate optimized symbol"
    echo "- equal-length bytes route through subtle::ConstantTimeEq"
    echo "- no memcmp or bcmp call is emitted in the companion assembly"
    echo "- length mismatch and final Choice declassification remain public"
} >"$manifest"

echo "subtle asm evidence: wrote $output_dir"
