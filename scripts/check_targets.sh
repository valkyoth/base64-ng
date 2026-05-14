#!/usr/bin/env sh
set -eu

targets="${*:-x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf}"
installed="$(rustup target list --installed)"

for target in $targets; do
    if printf '%s\n' "$installed" | grep -qx "$target"; then
        echo "target checks: no_std simd-reserved build for $target"
        cargo check --target "$target" --no-default-features --features simd --lib
    else
        echo "target checks: skipping $target; Rust target is not installed"
    fi
done
