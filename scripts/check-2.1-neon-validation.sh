#!/usr/bin/env sh
set -eu

mode="${1:-}"
case "$mode" in ''|--qemu) ;; *) echo "usage: $0 [--qemu]" >&2; exit 1 ;; esac
python3 scripts/test-neon-validation-codegen.py
python3 scripts/test-2.0-skeleton.py
host="$(rustc -vV | sed -n 's/^host: //p')"
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
execute=0
case "$host" in
    aarch64-*) target="$host"; execute=1 ;;
    *) target=aarch64-unknown-linux-musl
       export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld ;;
esac
if [ "$mode" = --qemu ]; then
    target=aarch64-unknown-linux-musl
    command -v qemu-aarch64 >/dev/null
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUNNER='qemu-aarch64 -cpu max,sve=off'
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld
    execute=1
    echo "NEON validation: QEMU functional evidence only, not native hardware"
fi

for compiler in "$active" 1.90.0; do
    if ! rustup run "$compiler" rustc --version >/dev/null 2>&1; then
        rustup toolchain install "$compiler" --profile minimal --component clippy
    fi
    rustup component add --toolchain "$compiler" clippy
    if ! rustup target list --installed --toolchain "$compiler" | grep -F -x -q "$target"; then
        rustup target add --toolchain "$compiler" "$target"
    fi
    for features in '' simd simd,checked-backend std,simd std,simd,checked-backend; do
        set -- --lib
        case "$features" in std,*) set -- --lib --tests ;; esac
        cargo +"$compiler" clippy --locked --target "$target" --no-default-features \
            --features "$features" "$@" -- -D warnings
        case "$features" in
            std,*)
                if [ "$execute" -eq 1 ]; then
                    BASE64_NG_REQUIRE_NEON_VALIDATION=1 \
                    cargo +"$compiler" test --locked --release --target "$target" \
                        --no-default-features --features "$features" --lib neon_candidate -- --test-threads=1
                    cargo +"$compiler" test --locked --release --target "$target" \
                        --no-default-features --features "$features" --lib neon_validation_guard -- --test-threads=1
                fi ;;
        esac
    done
done
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM
CARGO_TARGET_DIR="$tmp/production" cargo rustc --locked --release --target "$target" \
    --all-features --lib -- --emit=llvm-ir
python3 scripts/check-neon-validation-codegen.py "$tmp/production/$target/release/deps" production
CARGO_TARGET_DIR="$tmp/test" cargo rustc --locked --release --target "$target" \
    --all-features --lib -- --emit=asm --test
python3 scripts/check-neon-validation-codegen.py "$tmp/test/$target/release/deps" assembly
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
if [ "$execute" -eq 0 ]; then
    echo "NEON validation: cross-build/codegen passed; execution not run (use --qemu or native ARM)"
else
    echo "NEON validation: execution, feature/MSRV and codegen checks passed ($host; mode=${mode:-native})"
fi
