#!/usr/bin/env sh
set -eu

. scripts/evidence-source.sh
evidence_capture_source "final release evidence"

root="target/release-evidence"
manifest="$root/FINAL-MANIFEST.txt"
manifest_signature="$manifest.sig"
equivalence_manifest="$root/EQUIVALENCE-MANIFEST.txt"
campaign_commit="${BASE64_NG_REUSE_EVIDENCE_FROM:-$EVIDENCE_SOURCE_COMMIT}"
external_campaign_commit="${BASE64_NG_CAMPAIGN_SOURCE_COMMIT:-}"
mkdir -p "$root"
if [ -L "$root" ] || [ ! -d "$root" ] || \
    [ -n "$(find "$root" -type l -print -quit)" ]; then
    echo "final release evidence: evidence tree contains a symbolic link" >&2
    exit 1
fi

if [ "$campaign_commit" != "$EVIDENCE_SOURCE_COMMIT" ]; then
    require_retained_manifest="$manifest"
    if [ ! -s "$require_retained_manifest" ]; then
        echo "final release evidence: retained FINAL-MANIFEST is required for evidence reuse" >&2
        exit 1
    fi
    python3 scripts/evidence-equivalence.py \
        --evidence-commit "$campaign_commit" \
        --release-commit "$EVIDENCE_SOURCE_COMMIT" \
        --retained-manifest "$require_retained_manifest" \
        --output "$equivalence_manifest"
else
    rm -f "$equivalence_manifest"
fi
if [ -n "$external_campaign_commit" ]; then
    external_candidate="$EVIDENCE_SOURCE_COMMIT"
    if [ "$campaign_commit" != "$EVIDENCE_SOURCE_COMMIT" ]; then
        external_candidate="$campaign_commit"
    fi
    python3 scripts/validate-campaign-source-equivalence.py \
        --campaign "$external_campaign_commit" \
        --candidate "$external_candidate"
fi

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

require_source_manifest_for() {
    file="$1"
    expected_primary="$2"
    expected_secondary="${3:-}"
    require_file "$file"
    evidence_require_singleton_manifest_line \
        "$file" 'source:' 'source section' "final release evidence"
    evidence_require_exact_manifest_key \
        "$file" tree_state clean "final release evidence"
    actual_commit="$(sed -n 's/^commit=//p' "$file")"
    if [ "$actual_commit" != "$expected_primary" ] && \
        { [ -z "$expected_secondary" ] || [ "$actual_commit" != "$expected_secondary" ]; }
    then
        echo "final release evidence: invalid campaign commit in $file: ${actual_commit:-missing}" >&2
        exit 1
    fi
    evidence_require_exact_manifest_key \
        "$file" commit "$actual_commit" "final release evidence"
}

require_report_key() {
    file="$1"
    key="$2"
    expected="$3"
    require_file "$file"
    actual="$(awk -v key="$key" '
        index($0, key "=") == 1 {
            count += 1
            value = substr($0, length(key) + 2)
        }
        END { if (count == 1) print value; else exit 1 }
    ' "$file")" || {
        echo "final release evidence: report has no singleton $key: $file" >&2
        exit 1
    }
    if [ "$actual" != "$expected" ]; then
        echo "final release evidence: expected $key=$expected in $file, got $actual" >&2
        exit 1
    fi
}

for file in \
    "$root/miri/MANIFEST.txt" \
    "$root/2.0-memory-sanitizers/MANIFEST.txt" \
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
    "$root/commit-53/MANIFEST.txt"
do
    require_source_manifest_for "$file" "$campaign_commit" "$EVIDENCE_SOURCE_COMMIT"
done

if [ -n "$external_campaign_commit" ]; then
    require_source_manifest_for \
        "$root/fuzz/MANIFEST.txt" "$external_campaign_commit"
else
    require_source_manifest_for \
        "$root/fuzz/MANIFEST.txt" "$campaign_commit" "$EVIDENCE_SOURCE_COMMIT"
fi

require_report_key "$root/big-endian-qemu/report.txt" source_commit "$campaign_commit"
require_report_key "$root/big-endian-qemu/report.txt" s390x_result pass
require_report_key "$root/big-endian-qemu/report.txt" powerpc64_result pass
require_report_key "$root/riscv-qemu/report.txt" source_commit "$campaign_commit"
require_report_key "$root/riscv-qemu/report.txt" result pass
require_report_key "$root/sve-qemu/report.txt" source_commit "$campaign_commit"
require_report_key "$root/sve-qemu/report.txt" result pass

rvv_native="$root/riscv-native-admission"
scripts/validate-rvv-admission-bundle.py "$rvv_native"
if [ -n "$(find "$rvv_native" -type l -print -quit)" ]; then
    echo "final release evidence: native RVV bundle contains a symbolic link" >&2
    exit 1
fi
require_report_key "$rvv_native/MANIFEST.txt" source_commit \
    "${external_campaign_commit:-$campaign_commit}"
require_report_key \
    "$rvv_native/MANIFEST.txt" execution_environment real-hardware
require_report_key \
    "$rvv_native/MANIFEST.txt" admission_scope linux-rvv-1.0-vlen256-spacemit-x60

# Package composition can change when release-process files change. These
# artifacts are therefore always regenerated for the tag candidate even when
# expensive runtime campaigns are reused.
for file in \
    "$root/sbom-MANIFEST.txt" \
    "$root/reproducible/MANIFEST.txt"
do
    require_source_manifest_for "$file" "$EVIDENCE_SOURCE_COMMIT"
done

require_file "$root/base64-ng.spdx.json"
require_file "$root/base64-ng.cyclonedx.json"
scripts/validate-release-evidence-outcomes.sh "$root"

evidence_verify_source "final release evidence"

files="$(
    find "$root" -type f \
        ! -path "$manifest" \
        ! -path "$manifest_signature" \
        ! -path "$manifest_tmp" \
        -print | LC_ALL=C sort
)"
if [ -z "$files" ]; then
    echo "final release evidence: no evidence artifacts found" >&2
    exit 1
fi

{
    echo "base64-ng final release evidence index"
    echo
    evidence_write_source_manifest
    echo
    if [ "$campaign_commit" = "$EVIDENCE_SOURCE_COMMIT" ]; then
        echo "evidence_mode=exact"
    else
        echo "evidence_mode=metadata-equivalent"
    fi
    echo "campaign_commit=$campaign_commit"
    echo "release_commit=$EVIDENCE_SOURCE_COMMIT"
    if [ -n "$external_campaign_commit" ]; then
        echo "external_campaign_commit=$external_campaign_commit"
        echo "fuzz_campaign_commit=$external_campaign_commit"
        echo "rvv_campaign_commit=$external_campaign_commit"
    fi
    echo
    echo "required_campaigns=miri,sanitizers,fuzz-release,dudect-release,kani-normal,kani-advanced,assembly,native-neon,native-rvv,sbom"
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
echo "final release evidence: manifest remains unsigned until the isolated sealing step"
