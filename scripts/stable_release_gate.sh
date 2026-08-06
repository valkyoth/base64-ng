#!/usr/bin/env sh
set -eu

mode="${1:-check}"
reuse_evidence_from="${BASE64_NG_REUSE_EVIDENCE_FROM:-}"

case "$mode" in
    check | candidate | release)
        ;;
    *)
        echo "usage: scripts/stable_release_gate.sh [check|candidate|release]" >&2
        exit 2
        ;;
esac

if [ "$mode" = "check" ]; then
    # Never leak the development-only override into policy self-tests. Evidence
    # commands receive it individually through run_evidence below.
    unset BASE64_NG_ALLOW_DIRTY_EVIDENCE
else
    unset BASE64_NG_ALLOW_DIRTY_EVIDENCE
    BASE64_NG_RUN_COMMIT54_PUBLISH_DRY_RUN=1
    export BASE64_NG_RUN_COMMIT54_PUBLISH_DRY_RUN
    . scripts/evidence-source.sh
    evidence_capture_source "stable release gate"
fi

if [ "$mode" = "check" ] && [ -n "$reuse_evidence_from" ]; then
    echo "stable release gate: evidence reuse applies only to candidate or release mode" >&2
    exit 2
fi

run_evidence() {
    if [ "$mode" = "check" ]; then
        BASE64_NG_ALLOW_DIRTY_EVIDENCE=1 "$@"
    else
        "$@"
    fi
}

cargo_version="$(
    sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"

if [ "$mode" != "check" ]; then
    case "$cargo_version" in
        *-*)
            echo "stable release gate: strict modes require a stable Cargo.toml version, got $cargo_version" >&2
            exit 1
            ;;
    esac
fi

echo "stable release gate: standard checks"
scripts/checks.sh

if [ "$mode" = "release" ]; then
    echo "stable release gate: completed 2.0 checkpoint record"
    scripts/validate-2.0-checkpoint-record.py --final
fi

if cargo nextest --version >/dev/null 2>&1; then
    echo "stable release gate: nextest"
    cargo nextest run --all-features
else
    echo "stable release gate: skipping nextest; cargo nextest is not installed"
fi

if [ -n "$reuse_evidence_from" ]; then
    echo "stable release gate: verify metadata-only retained evidence"
    python3 scripts/evidence-equivalence.py \
        --evidence-commit "$reuse_evidence_from" \
        --retained-manifest target/release-evidence/FINAL-MANIFEST.txt

    # Refresh relatively small evidence that was either affected by release
    # process packaging or may have been overwritten by ordinary dirty-tree
    # development checks. Long-running runtime campaigns retain their original
    # manifests and commit provenance.
    echo "stable release gate: refresh candidate-bound native inventory"
    BASE64_NG_REQUIRE_COMMIT53_NATIVE=1 \
        run_evidence scripts/check-2.0-memory-hardware-evidence.sh
    echo "stable release gate: refresh candidate-bound NEON assembly"
    run_evidence scripts/generate_neon_asm_evidence.sh
    echo "stable release gate: refresh candidate-bound wasm assembly"
    run_evidence scripts/generate_wasm_simd_evidence.sh

    echo "stable release gate: validate retained campaign outcomes"
    scripts/validate-release-evidence-outcomes.sh target/release-evidence
    echo "stable release gate: regenerate candidate SBOM"
    run_evidence scripts/generate-sbom.sh
    echo "stable release gate: regenerate candidate package/build evidence"
    run_evidence scripts/reproducible_build_check.sh

    evidence_verify_source "stable release gate"
    echo "stable release gate: metadata-equivalent evidence index"
    scripts/finalize-release-evidence.sh
    evidence_verify_source "stable release gate"

    if [ "$mode" = "release" ]; then
        echo "stable release gate: final pentest report"
        scripts/validate-release-readiness.sh "v${cargo_version}"
        evidence_verify_source "stable release gate"
    fi

    echo "stable release gate: ok ($mode, metadata-equivalent evidence)"
    exit 0
fi

echo "stable release gate: Miri"
if [ "$mode" = "check" ]; then
    run_evidence scripts/check_miri.sh
else
    BASE64_NG_REQUIRE_MIRI=1 run_evidence scripts/check_miri.sh
fi

echo "stable release gate: address, leak, and thread sanitizers"
if [ "$mode" = "check" ]; then
    run_evidence scripts/check-2.0-in-place-sanitizers.sh
else
    BASE64_NG_REQUIRE_SANITIZERS=1 \
        run_evidence scripts/check-2.0-in-place-sanitizers.sh
fi

echo "stable release gate: final native hardware evidence"
if [ "$mode" != "check" ]; then
    BASE64_NG_REQUIRE_COMMIT53_NATIVE=1 \
        run_evidence scripts/check-2.0-memory-hardware-evidence.sh
else
    run_evidence scripts/check-2.0-memory-hardware-evidence.sh
fi

if [ "$mode" = "check" ]; then
    echo "stable release gate: fuzz compile and policy checks"
    scripts/check_fuzz.sh
elif [ -n "${BASE64_NG_FUZZ_SHARD_DIR:-}" ]; then
    echo "stable release gate: verified distributed release-duration fuzz campaigns"
    run_evidence scripts/aggregate-fuzz-shards.sh "$BASE64_NG_FUZZ_SHARD_DIR"
else
    echo "stable release gate: release-duration fuzz campaigns"
    BASE64_NG_RUN_FUZZ_RELEASE=1 BASE64_NG_FUZZ_SECONDS_PER_TARGET=3600 \
        run_evidence scripts/check_fuzz.sh
fi

echo "stable release gate: dudect timing evidence"
if [ "$mode" = "check" ]; then
    scripts/check_dudect.sh
else
    BASE64_NG_RUN_DUDECT=1 BASE64_NG_DUDECT_RELEASE=1 \
        BASE64_NG_DUDECT_THRESHOLD=10 \
        run_evidence scripts/check_dudect.sh
fi

echo "stable release gate: installed cross-target checks"
scripts/check_targets.sh

echo "stable release gate: big-endian QEMU checks"
scripts/check_big_endian_qemu.sh --all

echo "stable release gate: RISC-V QEMU checks"
scripts/check_riscv_qemu.sh

echo "stable release gate: RVV candidate assembly evidence"
run_evidence scripts/generate_rvv_asm_evidence.sh

echo "stable release gate: AArch64 SVE QEMU checks"
scripts/check_sve_qemu.sh

echo "stable release gate: SVE candidate assembly evidence"
run_evidence scripts/generate_sve_asm_evidence.sh

echo "stable release gate: no-alloc portability smoke"
scripts/check_no_alloc_smoke.sh

echo "stable release gate: migration guide smoke"
scripts/check_migration_smoke.sh

echo "stable release gate: SIMD feature-bundle checks"
scripts/check_simd_feature_bundles.sh

echo "stable release gate: backend evidence"
run_evidence scripts/check_backend_evidence.sh

echo "stable release gate: Kani proofs"
if [ "$mode" = "check" ]; then
    run_evidence scripts/check_kani.sh
else
    BASE64_NG_REQUIRE_KANI=1 run_evidence scripts/check_kani.sh
    BASE64_NG_REQUIRE_KANI=1 BASE64_NG_KANI_ALL_ADVANCED=1 \
        run_evidence scripts/check_kani_advanced.sh
fi

echo "stable release gate: timing and generated-code boundaries"
scripts/validate-2.0-timing-boundaries.sh

echo "stable release gate: constant-time assembly evidence"
run_evidence scripts/generate_ct_asm_evidence.sh

echo "stable release gate: SIMD assembly evidence"
run_evidence scripts/generate_simd_asm_evidence.sh

echo "stable release gate: wasm SIMD codegen evidence"
run_evidence scripts/generate_wasm_simd_evidence.sh

echo "stable release gate: SBOM"
run_evidence scripts/generate-sbom.sh

echo "stable release gate: reproducible package/build"
run_evidence scripts/reproducible_build_check.sh

if [ "$mode" != "check" ]; then
    evidence_verify_source "stable release gate"
    echo "stable release gate: exact-candidate evidence index"
    scripts/finalize-release-evidence.sh
    evidence_verify_source "stable release gate"
fi

if [ "$mode" = "release" ]; then
    echo "stable release gate: final pentest report"
    scripts/validate-release-readiness.sh "v${cargo_version}"
    evidence_verify_source "stable release gate"
fi

echo "stable release gate: ok ($mode)"
