#!/usr/bin/env sh
set -eu

target="${1:-x86_64-unknown-linux-gnu}"
installed="$(rustup target list --installed)"

if ! printf '%s\n' "$installed" | grep -qx "$target"; then
    echo "simd feature-bundle checks: skipping $target; Rust target is not installed"
    exit 0
fi

echo "simd feature-bundle checks: AVX2 no_std reserved build for $target"
RUSTFLAGS='-C target-feature=+avx2' \
    cargo check --target "$target" --no-default-features --features simd --lib

echo "simd feature-bundle checks: AVX-512 VBMI no_std reserved build for $target"
RUSTFLAGS='-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi' \
    cargo check --target "$target" --no-default-features --features simd --lib
