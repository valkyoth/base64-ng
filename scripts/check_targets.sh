#!/usr/bin/env sh
set -eu

targets="${*:-x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-unknown-freebsd riscv64gc-unknown-linux-gnu riscv32imac-unknown-none-elf wasm32-unknown-unknown thumbv7em-none-eabihf}"
installed="$(rustup target list --installed)"

for target in $targets; do
    if printf '%s\n' "$installed" | grep -qx "$target"; then
        echo "target checks: no_std simd-reserved build for $target"
        features="simd"
        case "$target" in
            wasm32-unknown-unknown)
                features="simd,allow-wasm32-best-effort-wipe"
                ;;
            s390x-unknown-linux-gnu | powerpc64-unknown-linux-gnu)
                features="simd,allow-compiler-fence-only-wipe"
                ;;
        esac
        cargo check --target "$target" --no-default-features --features "$features" --lib
    else
        echo "target checks: skipping $target; Rust target is not installed"
    fi
done
