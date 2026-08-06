#!/usr/bin/env sh
set -eu
ulimit -c 0

. scripts/evidence-source.sh
evidence_capture_source "RISC-V QEMU evidence"

script_revision="2026-08-06-signed-provenance-v4"
evidence_dir="target/release-evidence/riscv-qemu"
target="riscv64gc-unknown-linux-gnu"

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "RISC-V QEMU checks: missing command: $1" >&2
        echo "RISC-V QEMU checks: install hint: $2" >&2
        exit 1
    fi
}

require_target() {
    if ! rustup target list --installed | grep -qx "$target"; then
        toolchain="$(rustup show active-toolchain | sed 's/ .*//')"
        echo "RISC-V QEMU checks: missing Rust target: $target" >&2
        echo "  rustup target add --toolchain $toolchain $target" >&2
        exit 1
    fi
    rustc --print cfg --target "$target" | grep -q 'target_arch="riscv64"'
    rustc --print cfg --target "$target" | grep -q 'target_endian="little"'
}

detect_toolchain() {
    if [ -n "${BASE64_NG_RISCV64_LINKER:-}" ]; then
        linker="$BASE64_NG_RISCV64_LINKER"
        sysroot="${BASE64_NG_RISCV64_SYSROOT:?set BASE64_NG_RISCV64_SYSROOT with a custom linker}"
        libdirs="${BASE64_NG_RISCV64_LIBDIRS:-/lib64:/lib64/lp64d:/usr/lib64:/usr/lib64/lp64d:/lib:/usr/lib}"
        return
    fi
    if command -v riscv64-suse-linux-gcc >/dev/null 2>&1; then
        linker="riscv64-suse-linux-gcc"
        sysroot="/usr/riscv64-suse-linux/sys-root"
        libdirs="/lib64:/lib64/lp64d:/usr/lib64:/usr/lib64/lp64d"
        return
    fi
    if command -v riscv64-linux-gnu-gcc >/dev/null 2>&1; then
        linker="riscv64-linux-gnu-gcc"
        sysroot="/usr/riscv64-linux-gnu"
        libdirs="/lib:/usr/lib"
        return
    fi
    echo "RISC-V QEMU checks: no supported riscv64 cross linker found" >&2
    echo "RISC-V QEMU checks: install riscv64 gcc/glibc cross packages" >&2
    exit 1
}

require_sysroot() {
    found_crt=0
    for candidate in \
        "$sysroot/usr/lib64/lp64d/Scrt1.o" \
        "$sysroot/usr/lib/Scrt1.o" \
        "$sysroot/lib/Scrt1.o"
    do
        if [ -e "$candidate" ]; then
            found_crt=1
            break
        fi
    done
    if [ "$found_crt" -ne 1 ]; then
        echo "RISC-V QEMU checks: incomplete sysroot; Scrt1.o not found below $sysroot" >&2
        exit 1
    fi
}

target_key="RISCV64GC_UNKNOWN_LINUX_GNU"
runner_for_scalar() {
    printf '%s' "qemu-riscv64 -cpu rv64,v=false -L $sysroot -E LD_LIBRARY_PATH=$libdirs"
}

runner_for_vlen() {
    vlen="$1"
    printf '%s' "qemu-riscv64 -cpu rv64,v=true,vext_spec=v1.0,vlen=$vlen,elen=64 -L $sysroot -E LD_LIBRARY_PATH=$libdirs"
}

cargo_target() {
    runner="$1"
    shift
    env \
        "BASE64_NG_TEST_SUBPROCESS_RUNNER=$runner" \
        "CARGO_TARGET_${target_key}_LINKER=$linker" \
        "CARGO_TARGET_${target_key}_RUNNER=$runner" \
        cargo "$@"
}

cargo_candidate() {
    runner="$1"
    shift
    candidate_flags="${RUSTFLAGS:-} --cfg base64_ng_rvv_candidate"
    env \
        RUSTFLAGS="$candidate_flags" \
        "BASE64_NG_TEST_SUBPROCESS_RUNNER=$runner" \
        "CARGO_TARGET_${target_key}_LINKER=$linker" \
        "CARGO_TARGET_${target_key}_RUNNER=$runner" \
        cargo "$@"
}

echo "RISC-V QEMU checks: script=$script_revision"
echo "RISC-V QEMU checks: host=$(rustc -vV | sed -n 's/^host: //p')"
echo "RISC-V QEMU checks: rustc=$(rustc --version)"
echo "RISC-V QEMU checks: cargo=$(cargo --version)"

require_command qemu-riscv64 "install qemu-user or qemu-linux-user"
require_target
detect_toolchain
require_command "$linker" "install the selected riscv64 cross compiler"
require_sysroot

runner_scalar="$(runner_for_scalar)"
runner_128="$(runner_for_vlen 128)"
runner_256="$(runner_for_vlen 256)"
echo "RISC-V QEMU checks: linker=$linker"
echo "RISC-V QEMU checks: sysroot=$sysroot"

echo "RISC-V QEMU checks: scalar/fallback default-feature suite"
cargo_target "$runner_scalar" test --target "$target" --lib --tests -- --test-threads=1

echo "RISC-V QEMU checks: scalar/fallback all-feature suite"
cargo_target "$runner_scalar" test --target "$target" --all-features --lib --tests -- --test-threads=1

echo "RISC-V QEMU checks: scalar/fallback no-default suite"
cargo_target "$runner_scalar" test --target "$target" --no-default-features --lib --tests -- --test-threads=1

echo "RISC-V QEMU checks: all-feature doctests"
cargo_target "$runner_scalar" test --target "$target" --all-features --doc -- --test-threads=1

echo "RISC-V QEMU checks: no-default doctests"
cargo_target "$runner_scalar" test --target "$target" --no-default-features --doc -- --test-threads=1

echo "RISC-V QEMU checks: RVV candidate VLEN=128"
cargo_candidate "$runner_128" test --target "$target" --all-features --lib rvv_ -- --nocapture --test-threads=1

echo "RISC-V QEMU checks: RVV candidate VLEN=256"
cargo_candidate "$runner_256" test --target "$target" --all-features --lib rvv_ -- --nocapture --test-threads=1

echo "RISC-V QEMU checks: no_std static +v candidate compile"
candidate_static_flags="${RUSTFLAGS:-} --cfg base64_ng_rvv_candidate -C target-feature=+v"
env \
    RUSTFLAGS="$candidate_static_flags" \
    "CARGO_TARGET_${target_key}_LINKER=$linker" \
    cargo check --target "$target" --no-default-features --features simd --lib

evidence_verify_source "RISC-V QEMU evidence"

mkdir -p "$evidence_dir"
{
    echo "base64-ng RISC-V QEMU evidence"
    echo "script=$script_revision"
    echo "source_commit=$EVIDENCE_SOURCE_COMMIT"
    echo "tree_state=$EVIDENCE_TREE_STATE"
    echo "result=pass"
    echo "target=$target"
    echo "linker=$linker"
    echo "sysroot=$sysroot"
    echo "qemu=$(qemu-riscv64 --version | sed -n '1p')"
    echo "rustc=$(rustc --version)"
    echo "cargo=$(cargo --version)"
    echo "scalar_cpu=rv64,v=false"
    echo "candidate_vlens=128,256"
    echo "candidate_isa=RVV-1.0-basic-integer"
    echo "qemu_test_threads=1"
    echo "candidate_dispatch=internal-QEMU-only"
    echo "public_dispatch=scalar"
    echo "evidence_scope=scalar functional coverage plus non-admitted RVV encode/decode candidate correctness"
    echo "not_evidence_for=real hardware correctness, performance, timing, ABI preservation, or production admission"
    echo "hardware_status=real RVV hardware evidence required before public dispatch admission"
} >"$evidence_dir/report.txt"

evidence_verify_source "RISC-V QEMU evidence"
echo "RISC-V QEMU checks: wrote $evidence_dir/report.txt"
echo "RISC-V QEMU checks: ok"
