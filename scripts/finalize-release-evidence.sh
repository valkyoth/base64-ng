#!/usr/bin/env sh
set -eu

. scripts/evidence-source.sh
evidence_capture_source "final release evidence"

root="target/release-evidence"
manifest="$root/FINAL-MANIFEST.txt"
mkdir -p "$root"
rm -f "$manifest"
manifest_tmp="$(mktemp "$root/.FINAL-MANIFEST.XXXXXX")"

cleanup_manifest() {
    rm -f "$manifest_tmp"
}
trap cleanup_manifest EXIT INT TERM

require_file() {
    if [ ! -s "$1" ]; then
        echo "final release evidence: missing or empty artifact: $1" >&2
        exit 1
    fi
}

require_source_manifest() {
    file="$1"
    require_file "$file"
    if ! grep -F -q "commit=$EVIDENCE_SOURCE_COMMIT" "$file" ||
        ! grep -F -q 'tree_state=clean' "$file"
    then
        echo "final release evidence: stale or dirty source manifest: $file" >&2
        exit 1
    fi
}

for file in \
    "$root/miri/MANIFEST.txt" \
    "$root/2.0-memory-sanitizers/MANIFEST.txt" \
    "$root/fuzz/MANIFEST.txt" \
    "$root/dudect/MANIFEST.txt" \
    "$root/backend/MANIFEST.txt" \
    "$root/kani/normal/source.txt" \
    "$root/kani/advanced/source.txt" \
    "$root/asm/MANIFEST.txt" \
    "$root/simd-asm/MANIFEST.txt" \
    "$root/neon-asm/MANIFEST.txt" \
    "$root/rvv-asm/MANIFEST.txt" \
    "$root/sve-asm/MANIFEST.txt" \
    "$root/wasm-simd/MANIFEST.txt" \
    "$root/commit-53/MANIFEST.txt" \
    "$root/sbom-MANIFEST.txt" \
    "$root/reproducible/MANIFEST.txt"
do
    require_source_manifest "$file"
done

require_file "$root/base64-ng.spdx.json"
require_file "$root/base64-ng.cyclonedx.json"
scripts/validate-release-evidence-outcomes.sh "$root"

evidence_verify_source "final release evidence"

files="$(find "$root" -type f ! -path "$manifest" ! -path "$manifest_tmp" -print | LC_ALL=C sort)"
if [ -z "$files" ]; then
    echo "final release evidence: no evidence artifacts found" >&2
    exit 1
fi

{
    echo "base64-ng final release evidence index"
    echo
    evidence_write_source_manifest
    echo
    echo "required_campaigns=miri,sanitizers,fuzz-release,dudect-release,kani-normal,kani-advanced,assembly,native-hardware,sbom"
    echo
    echo "artifact-hashes:"
    printf '%s\n' "$files" | while IFS= read -r file; do
        evidence_checksum_file "$file"
    done
} >"$manifest_tmp"

evidence_verify_source "final release evidence"
mv "$manifest_tmp" "$manifest"
trap - EXIT INT TERM
echo "final release evidence: wrote $manifest"
