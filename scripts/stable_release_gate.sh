#!/usr/bin/env sh
set -eu

mode="${1:-check}"

case "$mode" in
    check | release)
        ;;
    *)
        echo "usage: scripts/stable_release_gate.sh [check|release]" >&2
        exit 2
        ;;
esac

echo "stable release gate: standard checks"
scripts/checks.sh

if cargo nextest --version >/dev/null 2>&1; then
    echo "stable release gate: nextest"
    cargo nextest run --all-features
else
    echo "stable release gate: skipping nextest; cargo nextest is not installed"
fi

echo "stable release gate: Miri"
scripts/check_miri.sh

if cargo fuzz --version >/dev/null 2>&1 && [ -d fuzz ]; then
    echo "stable release gate: fuzz target compile check"
    cargo +nightly fuzz build
else
    echo "stable release gate: skipping fuzz compile check; cargo fuzz or fuzz/ is not available"
fi

echo "stable release gate: fuzz dependency checks"
scripts/check_fuzz.sh

echo "stable release gate: performance harness dependency checks"
scripts/check_perf.sh

echo "stable release gate: installed cross-target checks"
scripts/check_targets.sh

echo "stable release gate: SIMD feature-bundle checks"
scripts/check_simd_feature_bundles.sh

echo "stable release gate: Kani proofs"
scripts/check_kani.sh

echo "stable release gate: SBOM"
scripts/generate-sbom.sh

echo "stable release gate: reproducible package/build"
scripts/reproducible_build_check.sh

echo "stable release gate: ok ($mode)"
