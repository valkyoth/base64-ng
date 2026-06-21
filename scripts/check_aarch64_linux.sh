#!/usr/bin/env sh
set -eu

if [ "$(uname -s)" != "Linux" ]; then
    echo "AArch64 Linux checks: this script must be run on Linux" >&2
    exit 1
fi

machine="$(uname -m)"
if [ "$machine" != "aarch64" ] && [ "$machine" != "arm64" ]; then
    echo "AArch64 Linux checks: this script must be run on an AArch64 machine" >&2
    echo "AArch64 Linux checks: observed machine=$machine" >&2
    exit 1
fi

toolchain="$(rustup show active-toolchain | sed 's/ .*//')"
rustc_path="$(rustup which --toolchain "$toolchain" rustc)"
host="$(rustup run "$toolchain" rustc -vV | sed -n 's/^host: //p')"
script_revision="2026-06-21-aarch64-linux-v1"

cargo_check() {
    RUSTC="$rustc_path" rustup run "$toolchain" cargo "$@"
}

echo "AArch64 Linux checks: script=$script_revision"
echo "AArch64 Linux checks: host=$host machine=$machine toolchain=$toolchain"
echo "AArch64 Linux checks: rustc=$(rustup run "$toolchain" rustc --version)"
echo "AArch64 Linux checks: cargo=$(rustup run "$toolchain" cargo --version)"
echo "AArch64 Linux checks: rustc path=$rustc_path"
echo "AArch64 Linux checks: uname=$(uname -a)"

case "$host" in
    aarch64-unknown-linux-gnu|aarch64-unknown-linux-musl)
        ;;
    *)
        echo "AArch64 Linux checks: unexpected Rust host triple: $host" >&2
        exit 1
        ;;
esac

if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
    echo "AArch64 Linux checks: CARGO_BUILD_TARGET=$CARGO_BUILD_TARGET is set; host checks force --target $host"
fi

echo "AArch64 Linux checks: host test default features"
cargo_check test --target "$host" --all-targets

echo "AArch64 Linux checks: host test all features"
cargo_check test --target "$host" --all-targets --all-features

echo "AArch64 Linux checks: host clippy all features"
cargo_check clippy --target "$host" --all-targets --all-features -- -D warnings

echo "AArch64 Linux checks: NEON block evidence"
cargo_check test --target "$host" --features simd neon_encode_block_matches_scalar_when_available -- --nocapture

echo "AArch64 Linux checks: backend evidence"
scripts/check_backend_evidence.sh

echo "AArch64 Linux checks: SIMD feature bundles"
scripts/check_simd_feature_bundles.sh

echo "AArch64 Linux checks: SIMD admission"
scripts/validate-simd-admission.sh
scripts/validate-unsafe-boundary.sh

echo "AArch64 Linux checks: ok"
