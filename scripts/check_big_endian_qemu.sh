#!/usr/bin/env sh
set -eu
ulimit -c 0

script_revision="2026-08-02-commit31-big-endian-qemu-v3"
evidence_dir="target/release-evidence/big-endian-qemu"
mode="${1:---all}"

case "$mode" in
    --all | --s390x | --powerpc64)
        ;;
    *)
        echo "usage: $0 [--all|--s390x|--powerpc64]" >&2
        exit 2
        ;;
esac

require_command() {
    command_name="$1"
    install_hint="$2"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "big-endian QEMU checks: missing command: $command_name" >&2
        echo "big-endian QEMU checks: install hint: $install_hint" >&2
        exit 1
    fi
}

first_command() {
    for candidate in "$@"; do
        if command -v "$candidate" >/dev/null 2>&1; then
            printf '%s\n' "$candidate"
            return
        fi
    done
    return 1
}

first_directory() {
    for candidate in "$@"; do
        if [ -d "$candidate" ]; then
            printf '%s\n' "$candidate"
            return
        fi
    done
    return 1
}

require_rust_target() {
    target="$1"
    if ! rustup target list --installed | grep -qx "$target"; then
        toolchain="$(rustup show active-toolchain | sed 's/ .*//')"
        echo "big-endian QEMU checks: missing Rust target: $target" >&2
        echo "big-endian QEMU checks: install with:" >&2
        echo "  rustup target add --toolchain $toolchain $target" >&2
        exit 1
    fi
}

require_big_endian_target() {
    target="$1"
    if ! rustc --print cfg --target "$target" | grep -q 'target_endian="big"'; then
        echo "big-endian QEMU checks: target is not big-endian: $target" >&2
        exit 1
    fi
}

require_linker_runtime() {
    label="$1"
    linker="$2"
    install_hint="$3"
    for required_file in Scrt1.o crti.o libc.so; do
        resolved="$($linker -print-file-name="$required_file")"
        if [ "$resolved" = "$required_file" ] || [ ! -e "$resolved" ]; then
            echo "big-endian QEMU checks: $label linker cannot resolve $required_file" >&2
            echo "big-endian QEMU checks: install hint: $install_hint" >&2
            exit 1
        fi
    done
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
        "BASE64_NG_TEST_SUBPROCESS_RUNNER=$runner" \
        "CARGO_TARGET_${key}_LINKER=$linker" \
        "CARGO_TARGET_${key}_RUNNER=$runner" \
        cargo "$@"
}

run_target_suite() {
    label="$1"
    target="$2"
    linker="$3"
    runner="$4"

    echo "big-endian QEMU checks: $label target=$target"
    echo "big-endian QEMU checks: $label linker=$linker"
    echo "big-endian QEMU checks: $label runner=$runner"

    require_rust_target "$target"
    require_big_endian_target "$target"

    echo "big-endian QEMU checks: $label no_std secret/SIMD portability build"
    cargo_for_target "$target" "$linker" "$runner" check \
        --target "$target" --no-default-features \
        --features simd,secrets,allow-compiler-fence-only-wipe --lib

    echo "big-endian QEMU checks: $label default-feature complete suite"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --lib --tests -- --test-threads=1

    echo "big-endian QEMU checks: $label all-feature complete suite"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --lib --tests -- --test-threads=1

    echo "big-endian QEMU checks: $label no-default-feature complete suite"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --no-default-features --lib --tests -- --test-threads=1

    echo "big-endian QEMU checks: $label all-feature doctests"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --doc -- --test-threads=1

    echo "big-endian QEMU checks: $label no-default-feature doctests"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --no-default-features --doc -- --test-threads=1
}

run_s390x() {
    linker="${BASE64_NG_S390X_LINKER:-}"
    if [ -z "$linker" ]; then
        linker="$(first_command s390x-suse-linux-gcc s390x-linux-gnu-gcc || true)"
    fi
    if [ -z "$linker" ]; then
        echo "big-endian QEMU checks: no s390x cross linker found" >&2
        exit 1
    fi
    sysroot="${BASE64_NG_S390X_SYSROOT:-}"
    if [ -z "$sysroot" ]; then
        sysroot="$(first_directory /usr/s390x-suse-linux/sys-root /usr/s390x-linux-gnu || true)"
    fi
    if [ -z "$sysroot" ]; then
        echo "big-endian QEMU checks: no s390x runtime sysroot found" >&2
        exit 1
    fi
    runner="${BASE64_NG_S390X_RUNNER:-qemu-s390x -L $sysroot}"
    require_command qemu-s390x "install qemu-user or qemu-linux-user"
    require_linker_runtime s390x "$linker" \
        "install an s390x cross compiler and matching libc development sysroot"
    run_target_suite s390x s390x-unknown-linux-gnu "$linker" "$runner"
    s390x_result=pass
    s390x_linker="$linker"
    s390x_runner="$runner"
}

run_powerpc64() {
    linker="${BASE64_NG_POWERPC64_LINKER:-}"
    if [ -z "$linker" ]; then
        linker="$(first_command powerpc64-suse-linux-gcc-16 powerpc64-linux-gnu-gcc || true)"
    fi
    if [ -z "$linker" ]; then
        echo "big-endian QEMU checks: no powerpc64 cross linker found" >&2
        exit 1
    fi
    sysroot="${BASE64_NG_POWERPC64_SYSROOT:-}"
    if [ -z "$sysroot" ]; then
        sysroot="$(first_directory /usr/powerpc64-suse-linux/sys-root /usr/powerpc64-linux-gnu || true)"
    fi
    if [ -z "$sysroot" ]; then
        echo "big-endian QEMU checks: no powerpc64 runtime sysroot found" >&2
        exit 1
    fi
    runner="${BASE64_NG_POWERPC64_RUNNER:-qemu-ppc64 -L $sysroot}"
    require_command qemu-ppc64 "install qemu-user or qemu-linux-user"
    require_linker_runtime powerpc64 "$linker" \
        "install a powerpc64 cross compiler and matching libc development sysroot"
    run_target_suite powerpc64 powerpc64-unknown-linux-gnu "$linker" "$runner"
    powerpc64_result=pass
    powerpc64_linker="$linker"
    powerpc64_runner="$runner"
}

echo "big-endian QEMU checks: script=$script_revision"
echo "big-endian QEMU checks: host=$(rustc -vV | sed -n 's/^host: //p')"
echo "big-endian QEMU checks: rustc=$(rustc --version)"
echo "big-endian QEMU checks: cargo=$(cargo --version)"

scripts/validate-big-endian-byte-order.sh

s390x_result=not-run
powerpc64_result=not-run
s390x_linker=not-run
s390x_runner=not-run
powerpc64_linker=not-run
powerpc64_runner=not-run

case "$mode" in
    --all)
        run_s390x
        run_powerpc64
        ;;
    --s390x)
        run_s390x
        ;;
    --powerpc64)
        run_powerpc64
        ;;
esac

mkdir -p "$evidence_dir"
{
    echo "base64-ng big-endian QEMU evidence"
    echo "script=$script_revision"
    echo "mode=$mode"
    echo "source_commit=$(git rev-parse HEAD)"
    echo "s390x_result=$s390x_result"
    echo "s390x_linker=$s390x_linker"
    echo "s390x_runner=$s390x_runner"
    echo "powerpc64_result=$powerpc64_result"
    echo "powerpc64_linker=$powerpc64_linker"
    echo "powerpc64_runner=$powerpc64_runner"
    echo "qemu_s390x=$(qemu-s390x --version 2>/dev/null | sed -n '1p' || true)"
    echo "qemu_powerpc64=$(qemu-ppc64 --version 2>/dev/null | sed -n '1p' || true)"
    echo "rustc=$(rustc --version)"
    echo "cargo=$(cargo --version)"
    echo "evidence_scope=functional correctness and scalar/fallback behavior under QEMU user-mode"
    echo "covered_surfaces=default,all-features,no-default-features,RFC4648,malformed,incremental,stream,in-place,wrapping,secret-cleanup,backend-reporting,doctests"
    echo "guest_test_execution=serial to avoid host-QEMU thread-scheduler instability without reducing test coverage"
    echo "not_evidence_for=real hardware performance, timing, microarchitectural behavior, register retention, physical cleanup, or side-channel behavior"
    echo "wipe_barrier_status=compiler-fence-only feature enabled for secret checks on unsupported big-endian architectures"
    echo "hardware_status=community real-hardware reports required before accelerated big-endian admission"
} >"$evidence_dir/report.txt"

if [ "$mode" = "--all" ] && { [ "$s390x_result" != pass ] || [ "$powerpc64_result" != pass ]; }; then
    echo "big-endian QEMU checks: --all did not complete both required targets" >&2
    exit 1
fi

echo "big-endian QEMU checks: wrote $evidence_dir/report.txt"
echo "big-endian QEMU checks: ok ($mode)"
