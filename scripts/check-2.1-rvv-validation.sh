#!/usr/bin/env sh
set -eu
ulimit -c 0
mode="${1:-}"
case "$mode" in ''|--qemu) ;; *) echo "usage: $0 [--qemu]" >&2; exit 1 ;; esac
target=riscv64gc-unknown-linux-gnu
host="$(rustc -vV | sed -n 's/^host: //p')"
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
execute=0
if [ "$host" = "$target" ] && [ -z "$mode" ]; then
    execute=1
elif command -v riscv64-suse-linux-gcc >/dev/null; then
    export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-suse-linux-gcc
    sysroot=/usr/riscv64-suse-linux/sys-root
    libdirs=/lib64:/lib64/lp64d:/usr/lib64:/usr/lib64/lp64d
elif command -v riscv64-linux-gnu-gcc >/dev/null; then
    export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc
    sysroot=/usr/riscv64-linux-gnu
    libdirs=/lib:/usr/lib
else
    echo "RVV validation: missing RISC-V cross linker" >&2; exit 1
fi
if [ "$mode" = --qemu ]; then
    command -v qemu-riscv64 >/dev/null
    execute=1
    echo "RVV validation: QEMU functional evidence only, NOT native admission"
fi
python3 scripts/test-rvv-validation-codegen.py
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM
for compiler in "$active" 1.90.0; do
    if ! rustup run "$compiler" rustc --version >/dev/null 2>&1; then
        rustup toolchain install "$compiler" --profile minimal --component clippy
    fi
    rustup component add --toolchain "$compiler" clippy
    rustup target add --toolchain "$compiler" "$target"
    for features in '' simd simd,checked-backend std,simd std,simd,checked-backend; do
        set -- --lib
        case "$features" in std,*) set -- --lib --tests ;; esac
        cargo +"$compiler" clippy --locked --target "$target" --no-default-features \
            --features "$features" "$@" -- -D warnings
        case "$features" in std,*) ;; *) continue ;; esac
        directory="$tmp/$compiler/$features"
        CARGO_TARGET_DIR="$directory" cargo +"$compiler" rustc --locked --release \
            --target "$target" --no-default-features --features "$features" --lib -- --emit=asm
        python3 scripts/check-rvv-validation-codegen.py "$directory/$target/release/deps"
        if [ "$execute" -eq 0 ]; then continue; fi
        if [ "$mode" = --qemu ]; then vlens='128 256'; else vlens=native; fi
        for vlen in $vlens; do
            if [ "$vlen" != native ]; then
                export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_RUNNER="qemu-riscv64 -cpu rv64,v=true,vext_spec=v1.0,vlen=$vlen,elen=64 -L $sysroot -E LD_LIBRARY_PATH=$libdirs"
            fi
            RUSTFLAGS='--cfg base64_ng_rvv_candidate' cargo +"$compiler" test --locked --release \
                --target "$target" --no-default-features --features "$features" --lib \
                simd::rvv::ordinary -- --test-threads=1 --nocapture
        done
        if [ "$mode" = --qemu ]; then
            export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_RUNNER="qemu-riscv64 -cpu rv64,v=false -L $sysroot -E LD_LIBRARY_PATH=$libdirs"
        else
            export BASE64_NG_REQUIRE_X60=1
            cargo +"$compiler" test --locked --release --target "$target" --no-default-features \
                --features "$features" --lib simd::rvv::ordinary::tests::rvv_native_exact_profile_is_required \
                -- --ignored --exact
        fi
        cargo +"$compiler" test --locked --release --target "$target" --no-default-features \
            --features "$features" --test decode_validation
        cargo +"$compiler" test --locked --release --target "$target" --no-default-features \
            --features "$features" --lib ordinary_decode::vector -- --test-threads=1
        cargo +"$compiler" test --locked --release --target "$target" --no-default-features \
            --features "$features" --lib backend_health -- --test-threads=1
        cargo +"$compiler" test --locked --release --target "$target" --no-default-features \
            --features "$features" --lib ordinary_encode -- --test-threads=1
    done
done
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
if [ "$execute" -eq 0 ]; then
    echo "RVV validation: build/codegen passed; execution NOT run"
else
    echo "RVV validation: execution, feature/MSRV and production codegen passed ($mode; host=$host)"
fi
