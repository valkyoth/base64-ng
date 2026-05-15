#!/usr/bin/env sh
set -eu

x86_target="${1:-x86_64-unknown-linux-gnu}"
arm_target="${2:-aarch64-unknown-linux-gnu}"
wasm_target="${3:-wasm32-unknown-unknown}"
installed="$(rustup target list --installed)"

if printf '%s\n' "$installed" | grep -qx "$x86_target"; then
    echo "simd feature-bundle checks: AVX2 no_std reserved build for $x86_target"
    RUSTFLAGS='-C target-feature=+avx2' \
        cargo check --target "$x86_target" --no-default-features --features simd --lib

    echo "simd feature-bundle checks: AVX-512 VBMI no_std reserved build for $x86_target"
    RUSTFLAGS='-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi' \
        cargo check --target "$x86_target" --no-default-features --features simd --lib
else
    echo "simd feature-bundle checks: skipping $x86_target; Rust target is not installed"
fi

if printf '%s\n' "$installed" | grep -qx "$arm_target"; then
    echo "simd feature-bundle checks: NEON no_std reserved build for $arm_target"
    cargo check --target "$arm_target" --no-default-features --features simd --lib
else
    echo "simd feature-bundle checks: skipping $arm_target; Rust target is not installed"
fi

if printf '%s\n' "$installed" | grep -qx "$wasm_target"; then
    echo "simd feature-bundle checks: wasm simd128 no_std reserved build for $wasm_target"
    RUSTFLAGS='-C target-feature=+simd128' \
        cargo check --target "$wasm_target" --no-default-features --features simd --lib
else
    echo "simd feature-bundle checks: skipping $wasm_target; Rust target is not installed"
fi
