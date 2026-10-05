#!/usr/bin/env sh
set -eu

active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
runner="${WASMTIME:-$(command -v wasmtime || true)}"
if [ -z "$runner" ] && [ -x "$HOME/.wasmtime/bin/wasmtime" ]; then
    runner="$HOME/.wasmtime/bin/wasmtime"
fi

for toolchain in "$active" 1.90.0; do
    rustup component add clippy --toolchain "$toolchain"
    rustup target add wasm32-unknown-unknown --toolchain "$toolchain"
    for features in simd alloc,simd std,simd std,simd,checked-backend; do
        for flags in '' '-C target-feature=+simd128'; do
            RUSTFLAGS="$flags" cargo "+$toolchain" clippy --locked \
                --target wasm32-unknown-unknown --no-default-features --features "$features" -- -D warnings
        done
    done
    if [ -n "$runner" ]; then
        rustup target add wasm32-wasip1 --toolchain "$toolchain"
        export CARGO_TARGET_WASM32_WASIP1_RUNNER="$runner run -C cache=n"
        for features in std,simd std,simd,checked-backend; do
            for flags in '' '-C target-feature=+simd128'; do
                export RUSTFLAGS="$flags"
                cargo "+$toolchain" clippy --locked --target wasm32-wasip1 \
                    --no-default-features --features "$features" --lib --tests -- -D warnings
                for filter in simd::wasm::ordinary::tests v2::ordinary_decode::vector::tests ordinary_encode encode_backend::checked in_place_bulk_miri; do
                    cargo "+$toolchain" test --locked --release --target wasm32-wasip1 \
                        --no-default-features --features "$features" --lib "$filter" -- --test-threads=1
                done
                cargo "+$toolchain" test --locked --release --target wasm32-wasip1 \
                    --no-default-features --features "$features" --test decode_validation -- --test-threads=1
                cargo "+$toolchain" test --locked --release --target wasm32-wasip1 \
                    --no-default-features --features "$features" --test in_place_bulk -- --test-threads=1
            done
        done
        unset RUSTFLAGS
    fi
done
if [ -z "$runner" ]; then
    echo "WASM validation: compile matrix passed; execution SKIPPED (Wasmtime unavailable)"
else
    echo "WASM validation: active/MSRV scalar/SIMD, production-linked and fault-injection checks passed"
fi
