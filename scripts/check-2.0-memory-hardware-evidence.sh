#!/usr/bin/env sh
set -eu

document="docs/2.0_MEMORY_SANITIZER_HARDWARE_EVIDENCE.md"
evidence_root="target/release-evidence"
evidence_dir="$evidence_root/commit-53"
manifest="$evidence_dir/MANIFEST.txt"
mkdir -p "$evidence_dir"

. scripts/evidence-source.sh
evidence_capture_source "2.0 memory and hardware evidence"

for required in \
    'NEON performance | Retained native evidence' \
    'RVV is admitted only for the exact Linux SpacemiT X60 profile' \
    'Base 2.0 ships no persistent provider' \
    'Abort, `panic=abort`, OOM abort, process termination' \
    'No growth after' \
    'UndefinedBehaviorSanitizer is not a separately supported Rust sanitizer'
do
    if ! grep -F -q "$required" "$document"; then
        echo "2.0 memory/hardware evidence: missing documentation marker: $required" >&2
        exit 1
    fi
done

rvv_archive="$evidence_root/riscv-native-admission"
rvv_bundle="${BASE64_NG_RVV_ADMISSION_BUNDLE:-$rvv_archive}"
rvv_expected_commit="${BASE64_NG_EXPECTED_RVV_SOURCE_COMMIT:-$EVIDENCE_SOURCE_COMMIT}"
rvv_status="pending-native-admission"
if [ "${#rvv_expected_commit}" -ne 40 ]; then
    echo "2.0 memory/hardware evidence: expected RVV source is not a full commit" >&2
    exit 1
fi
case "$rvv_expected_commit" in
    *[!0-9a-f]*)
        echo "2.0 memory/hardware evidence: expected RVV source is not a full commit" >&2
        exit 1
        ;;
esac
if ! git cat-file -e "$rvv_expected_commit^{commit}" 2>/dev/null || \
    ! git merge-base --is-ancestor "$rvv_expected_commit" "$EVIDENCE_SOURCE_COMMIT"; then
    echo "2.0 memory/hardware evidence: expected RVV source is not an available ancestor" >&2
    exit 1
fi
if [ -e "$rvv_bundle" ]; then
    if [ ! -d "$rvv_bundle" ]; then
        echo "2.0 memory/hardware evidence: RVV bundle is not a directory: $rvv_bundle" >&2
        exit 1
    fi
    scripts/validate-rvv-admission-bundle.py "$rvv_bundle"
    rvv_source="$(sed -n 's/^source_commit=//p' "$rvv_bundle/MANIFEST.txt")"
    if [ "$rvv_source" = "$rvv_expected_commit" ]; then
        if [ "$rvv_bundle" != "$rvv_archive" ]; then
            rvv_temporary="$(mktemp -d "$evidence_root/.riscv-native-admission.XXXXXX")"
            if ! cp -R "$rvv_bundle"/. "$rvv_temporary"/; then
                rm -rf "$rvv_temporary"
                echo "2.0 memory/hardware evidence: failed to archive the RVV bundle" >&2
                exit 1
            fi
            if ! scripts/validate-rvv-admission-bundle.py "$rvv_temporary"; then
                rm -rf "$rvv_temporary"
                exit 1
            fi
            rm -rf "$rvv_archive"
            mv "$rvv_temporary" "$rvv_archive"
        fi
        rvv_status="exact-linux-spacemit-x60-native-admission"
    else
        echo "2.0 memory/hardware evidence: ignoring RVV bundle for unexpected commit $rvv_source"
    fi
fi
if [ "${BASE64_NG_REQUIRE_RVV_NATIVE:-0}" = "1" ] && \
    [ "$rvv_status" != "exact-linux-spacemit-x60-native-admission" ]; then
    echo "2.0 memory/hardware evidence: release requires exact-campaign native X60 RVV evidence" >&2
    echo "2.0 memory/hardware evidence: set BASE64_NG_RVV_ADMISSION_BUNDLE to the validated bundle" >&2
    exit 1
fi

echo "2.0 memory/hardware evidence: secret allocation identity"
cargo test --all-features --lib \
    'v2::secret_decoder_tests::vector_frame_never_reallocates_after_classified_input' \
    -- --exact
cargo test --all-features --lib \
    'v2::secret_encoder_tests::vector_encoder_never_reallocates_after_classified_input' \
    -- --exact

echo "2.0 memory/hardware evidence: volatile provider and combined limits"
cargo test --all-features --lib \
    'v2::assurance::tests::active_and_quarantined_allocations_share_every_provider_budget' \
    -- --exact
cargo test --all-features --lib \
    'v2::assurance::tests::fresh_volatile_provider_never_recovers_prior_instance_state' \
    -- --exact

echo "2.0 memory/hardware evidence: report schema tests"
scripts/test-big-endian-hardware-evidence.py
scripts/test-riscv-hardware-evidence.py
scripts/test-sve-hardware-evidence.py
scripts/test-perf-evidence.py

echo "2.0 memory/hardware evidence: retained x86 campaign"
scripts/validate-x86-encode-performance.py \
    performance-baselines/dispatch-commit-34-amd-9950x3d-linux/x86-encode.csv
scripts/validate-x86-decode-performance.py \
    performance-baselines/dispatch-commit-34-amd-9950x3d-linux/x86-decode.csv

neon_apple="performance-baselines/dispatch-2.0-neon-apple-silicon"
neon_linux="performance-baselines/dispatch-2.0-neon-aarch64-linux"
neon_status="pending-native-performance"
if [ -e "$neon_apple" ]; then
    scripts/validate-neon-admission-bundle.py \
        "$neon_apple" --platform apple-silicon
fi
if [ -e "$neon_linux" ]; then
    scripts/validate-neon-admission-bundle.py \
        "$neon_linux" --platform aarch64-linux
fi
if [ -d "$neon_apple" ] && [ -d "$neon_linux" ]; then
    apple_source="$(sed -n 's/^source_commit=//p' "$neon_apple/MANIFEST.txt")"
    linux_source="$(sed -n 's/^source_commit=//p' "$neon_linux/MANIFEST.txt")"
    if [ "$apple_source" != "$linux_source" ]; then
        echo "2.0 memory/hardware evidence: NEON bundles must test the same source commit" >&2
        exit 1
    fi
    neon_status="retained-native-performance"
fi
if [ "${BASE64_NG_REQUIRE_COMMIT53_NATIVE:-0}" = "1" ] && \
    [ "$neon_status" != "retained-native-performance" ]; then
    echo "2.0 memory/hardware evidence: release requires retained Apple and Linux NEON bundles" >&2
    exit 1
fi

evidence_verify_source "2.0 memory and hardware evidence"

{
    echo "base64-ng Commit 53 evidence inventory"
    evidence_write_source_manifest
    echo "rustc=$(rustc -V)"
    echo "cargo=$(cargo -V)"
    echo "host=$(rustc -vV | sed -n 's/^host: //p')"
    echo "x86_automatic_dispatch=retained-native-correctness-and-performance"
    echo "neon_automatic_dispatch=$neon_status"
    echo "wasm_simd128=runtime-and-browser-evidence-not-hardware-attestation"
    echo "big_endian=scalar-qemu-portability-native-report-optional"
    echo "rvv=$rvv_status"
    echo "sve=candidate-qemu-only-not-dispatchable"
    echo "persistent_provider=none"
    echo "miri=separate-release-artifact"
    echo "address_leak_thread_sanitizers=separate-release-artifacts"
    echo "semantic_corpus=semantic-corpus/v1/cases.tsv"
    echo "resource_schema=scripts/perf_evidence_schema.py"
} >"$manifest"

echo "2.0 memory/hardware evidence: wrote $manifest"
echo "2.0 memory/hardware evidence: ok ($neon_status)"
