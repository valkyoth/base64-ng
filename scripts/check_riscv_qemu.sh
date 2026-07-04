#!/usr/bin/env sh
set -eu

script_revision="2026-07-04-riscv-qemu-v1"
evidence_dir="target/release-evidence/riscv-qemu"
required_target="riscv64gc-unknown-linux-gnu"
required_linker="${BASE64_NG_RISCV64_LINKER:-riscv64-suse-linux-gcc}"
required_sysroot="${BASE64_NG_RISCV64_SYSROOT:-/usr/riscv64-suse-linux/sys-root}"
required_runner="${BASE64_NG_RISCV64_RUNNER:-qemu-riscv64 -L $required_sysroot -E LD_LIBRARY_PATH=/lib64:/lib64/lp64d:/usr/lib64:/usr/lib64/lp64d}"

require_command() {
    command_name="$1"
    install_hint="$2"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "RISC-V QEMU checks: missing command: $command_name" >&2
        echo "RISC-V QEMU checks: install hint: $install_hint" >&2
        exit 1
    fi
}

require_rust_target() {
    target="$1"
    if ! rustup target list --installed | grep -qx "$target"; then
        toolchain="$(rustup show active-toolchain | sed 's/ .*//')"
        echo "RISC-V QEMU checks: missing Rust target: $target" >&2
        echo "RISC-V QEMU checks: install with:" >&2
        echo "  rustup target add --toolchain $toolchain $target" >&2
        exit 1
    fi
}

require_little_endian_target() {
    target="$1"
    if ! rustc --print cfg --target "$target" | grep -q 'target_endian="little"'; then
        echo "RISC-V QEMU checks: target is not little-endian: $target" >&2
        exit 1
    fi
}

require_riscv_target() {
    target="$1"
    if ! rustc --print cfg --target "$target" | grep -q 'target_arch="riscv64"'; then
        echo "RISC-V QEMU checks: target is not riscv64: $target" >&2
        exit 1
    fi
}

require_glibc_sysroot() {
    sysroot="$1"
    libdir="$2"
    install_hint="$3"

    for required_file in Scrt1.o crti.o libc.so; do
        if [ ! -e "$sysroot/$libdir/$required_file" ]; then
            echo "RISC-V QEMU checks: sysroot is incomplete: missing $sysroot/$libdir/$required_file" >&2
            echo "RISC-V QEMU checks: install hint: $install_hint" >&2
            exit 1
        fi
    done

    if [ ! -e "$sysroot/lib64/libgcc_s.so.1" ] && [ ! -e "$sysroot/lib64/lp64d/libgcc_s.so.1" ]; then
        echo "RISC-V QEMU checks: sysroot is incomplete: missing target libgcc_s.so.1" >&2
        echo "RISC-V QEMU checks: install hint: $install_hint" >&2
        exit 1
    fi
}

target_key() {
    printf '%s' "$1" | tr '[:lower:]-' '[:upper:]_'
}

cargo_for_target() {
    target="$1"
    linker="$2"
    runner="$3"
    shift 3
    key="$(target_key "$target")"
    env \
        "CARGO_TARGET_${key}_LINKER=$linker" \
        "CARGO_TARGET_${key}_RUNNER=$runner" \
        cargo "$@"
}

run_target_suite() {
    target="$1"
    linker="$2"
    runner="$3"

    echo "RISC-V QEMU checks: target=$target"
    echo "RISC-V QEMU checks: linker=$linker"
    echo "RISC-V QEMU checks: runner=$runner"

    require_rust_target "$target"
    require_little_endian_target "$target"
    require_riscv_target "$target"

    echo "RISC-V QEMU checks: no_std simd-reserved library build"
    cargo_for_target "$target" "$linker" "$runner" check \
        --target "$target" --no-default-features \
        --features simd,allow-compiler-fence-only-wipe --lib

    echo "RISC-V QEMU checks: backend dispatch scalar fallback evidence"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --lib \
        backend_dispatch_matches_scalar_reference -- --nocapture

    echo "RISC-V QEMU checks: RFC4648 and buffer surface evidence"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --test rfc4648

    echo "RISC-V QEMU checks: strict SIMD decode dispatch surface evidence"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --test simd_decode_dispatch

    echo "RISC-V QEMU checks: stream surface evidence"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --test stream
}

echo "RISC-V QEMU checks: script=$script_revision"
echo "RISC-V QEMU checks: host=$(rustc -vV | sed -n 's/^host: //p')"
echo "RISC-V QEMU checks: rustc=$(rustc --version)"
echo "RISC-V QEMU checks: cargo=$(cargo --version)"

require_command qemu-riscv64 "sudo zypper install qemu-linux-user"
require_command "$required_linker" "sudo zypper install cross-riscv64-gcc16 cross-riscv64-binutils cross-riscv64-glibc-devel cross-riscv64-linux-glibc-devel"
require_glibc_sysroot \
    "$required_sysroot" \
    "usr/lib64/lp64d" \
    "install the matching riscv64 lp64d glibc-devel/sysroot package, or set BASE64_NG_RISCV64_SYSROOT to a complete sysroot"

run_target_suite "$required_target" "$required_linker" "$required_runner"

mkdir -p "$evidence_dir"
{
    echo "base64-ng RISC-V QEMU evidence"
    echo "script=$script_revision"
    echo "required_target=$required_target"
    echo "required_runner=$required_runner"
    echo "required_linker=$required_linker"
    echo "qemu_riscv64=$(qemu-riscv64 --version | sed -n '1p')"
    echo "rustc=$(rustc --version)"
    echo "cargo=$(cargo --version)"
    echo "evidence_scope=functional correctness and scalar/fallback behavior under QEMU user-mode"
    echo "not_evidence_for=real RVV hardware performance, timing, microarchitectural behavior, or side-channel behavior"
    echo "wipe_barrier_status=RISC-V cleanup uses an ordering fence; high-assurance speculative-execution deployments need platform mitigation"
    echo "hardware_status=community real-hardware reports requested for RVV 1.0 RISC-V systems such as SpacemiT K1/X60-class boards"
} >"$evidence_dir/report.txt"

echo "RISC-V QEMU checks: wrote $evidence_dir/report.txt"
echo "RISC-V QEMU checks: ok"
