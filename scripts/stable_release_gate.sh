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

if command -v cargo-nextest >/dev/null 2>&1; then
    echo "stable release gate: nextest"
    cargo nextest run --all-features
else
    echo "stable release gate: skipping nextest; cargo-nextest is not installed"
fi

if command -v cargo-fuzz >/dev/null 2>&1 && [ -d fuzz ]; then
    echo "stable release gate: fuzz target compile check"
    cargo +nightly fuzz build
else
    echo "stable release gate: skipping fuzz compile check; cargo-fuzz or fuzz/ is not available"
fi

if command -v cargo-kani >/dev/null 2>&1 && [ -d kani ]; then
    echo "stable release gate: Kani proofs"
    cargo kani
else
    echo "stable release gate: skipping Kani; cargo-kani or kani/ is not available"
fi

echo "stable release gate: SBOM"
scripts/generate-sbom.sh

echo "stable release gate: reproducible package/build"
scripts/reproducible_build_check.sh

echo "stable release gate: ok ($mode)"
