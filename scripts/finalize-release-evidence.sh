#!/usr/bin/env sh
set -eu

. scripts/evidence-source.sh
evidence_capture_source "final release evidence"

root="target/release-evidence"
manifest="$root/FINAL-MANIFEST.txt"

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

for file in \
    "$root/kani/normal/status.txt" \
    "$root/kani/advanced/status.txt"
do
    require_file "$file"
    if [ "$(sed -n '1p' "$file")" != "PASS" ]; then
        echo "final release evidence: required Kani status is not PASS: $file" >&2
        exit 1
    fi
done

require_file "$root/base64-ng.spdx.json"
require_file "$root/base64-ng.cyclonedx.json"

grep -F -q 'mode=release-duration' "$root/fuzz/MANIFEST.txt" || {
    echo "final release evidence: fuzz manifest is not a release-duration campaign" >&2
    exit 1
}

fuzz_duration="$(sed -n 's/^campaign_argument=-max_total_time=//p' "$root/fuzz/MANIFEST.txt")"
dudect_samples="$(sed -n 's/^samples=//p' "$root/dudect/MANIFEST.txt")"
dudect_iterations="$(sed -n 's/^iterations=//p' "$root/dudect/MANIFEST.txt")"
dudect_warmup="$(sed -n 's/^warmup=//p' "$root/dudect/MANIFEST.txt")"
for value in "$fuzz_duration" "$dudect_samples" "$dudect_iterations" "$dudect_warmup"; do
    case "$value" in
        '' | *[!0-9]*)
            echo "final release evidence: campaign manifest contains a missing or invalid numeric parameter" >&2
            exit 1
            ;;
    esac
done
if [ "$fuzz_duration" -lt 3600 ]; then
    echo "final release evidence: fuzz manifest lacks the one-hour-per-target floor" >&2
    exit 1
fi
if [ "$dudect_samples" -lt 20000 ] || [ "$dudect_iterations" -lt 64 ] || \
    [ "$dudect_warmup" -lt 1000 ]; then
    echo "final release evidence: dudect manifest lacks the release parameter floors" >&2
    exit 1
fi
grep -F -q 'neon_automatic_dispatch=retained-native-performance' \
    "$root/commit-53/MANIFEST.txt" || {
    echo "final release evidence: retained native NEON evidence is incomplete" >&2
    exit 1
}

evidence_verify_source "final release evidence"

files="$(find "$root" -type f ! -path "$manifest" -print | LC_ALL=C sort)"
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
} >"$manifest"

evidence_verify_source "final release evidence"
echo "final release evidence: wrote $manifest"
