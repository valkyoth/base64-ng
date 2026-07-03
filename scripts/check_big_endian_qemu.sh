#!/usr/bin/env sh
set -eu

script_revision="2026-07-03-big-endian-qemu-v1"
evidence_dir="target/release-evidence/big-endian-qemu"
required_target="s390x-unknown-linux-gnu"
required_linker="${BASE64_NG_S390X_LINKER:-s390x-suse-linux-gcc}"
required_runner="${BASE64_NG_S390X_RUNNER:-qemu-s390x -L /usr/s390x-suse-linux/sys-root}"
optional_powerpc64="${BASE64_NG_BIG_ENDIAN_RUN_POWERPC64:-0}"

require_command() {
    command_name="$1"
    install_hint="$2"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "big-endian QEMU checks: missing command: $command_name" >&2
        echo "big-endian QEMU checks: install hint: $install_hint" >&2
        exit 1
    fi
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
    label="$1"
    target="$2"
    linker="$3"
    runner="$4"

    echo "big-endian QEMU checks: $label target=$target"
    echo "big-endian QEMU checks: $label linker=$linker"
    echo "big-endian QEMU checks: $label runner=$runner"

    require_rust_target "$target"
    require_big_endian_target "$target"

    echo "big-endian QEMU checks: $label no_std simd-reserved library build"
    cargo_for_target "$target" "$linker" "$runner" check \
        --target "$target" --no-default-features \
        --features simd,allow-compiler-fence-only-wipe --lib

    echo "big-endian QEMU checks: $label backend dispatch scalar fallback evidence"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --lib \
        backend_dispatch_matches_scalar_reference -- --nocapture

    echo "big-endian QEMU checks: $label RFC4648 and buffer surface evidence"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --test rfc4648

    echo "big-endian QEMU checks: $label stream surface evidence"
    cargo_for_target "$target" "$linker" "$runner" test \
        --target "$target" --all-features --test stream
}

echo "big-endian QEMU checks: script=$script_revision"
echo "big-endian QEMU checks: host=$(rustc -vV | sed -n 's/^host: //p')"
echo "big-endian QEMU checks: rustc=$(rustc --version)"
echo "big-endian QEMU checks: cargo=$(cargo --version)"

require_command qemu-s390x "sudo zypper install qemu-linux-user"
require_command "$required_linker" "sudo zypper install cross-s390x-gcc16 cross-s390x-binutils cross-s390x-glibc-devel cross-s390x-linux-glibc-devel"

run_target_suite "s390x" "$required_target" "$required_linker" "$required_runner"

if [ "$optional_powerpc64" = "1" ]; then
    require_command qemu-ppc64 "sudo zypper install qemu-linux-user"
    require_command powerpc64-suse-linux-gcc-16 "sudo zypper install cross-ppc64-gcc16 cross-ppc64-binutils plus the matching powerpc64 glibc-devel/sysroot package"
    run_target_suite \
        "powerpc64" \
        "powerpc64-unknown-linux-gnu" \
        "powerpc64-suse-linux-gcc-16" \
        "qemu-ppc64 -L /usr/powerpc64-suse-linux/sys-root"
else
    echo "big-endian QEMU checks: skipping optional powerpc64; set BASE64_NG_BIG_ENDIAN_RUN_POWERPC64=1 to require it"
fi

mkdir -p "$evidence_dir"
{
    echo "base64-ng big-endian QEMU evidence"
    echo "script=$script_revision"
    echo "required_target=$required_target"
    echo "required_runner=$required_runner"
    echo "required_linker=$required_linker"
    echo "qemu_s390x=$(qemu-s390x --version | sed -n '1p')"
    echo "rustc=$(rustc --version)"
    echo "cargo=$(cargo --version)"
    echo "evidence_scope=functional correctness and scalar/fallback behavior under QEMU user-mode"
    echo "not_evidence_for=real hardware performance, timing, microarchitectural behavior, or side-channel behavior"
    echo "wipe_barrier_status=compiler-fence-only feature enabled for unsupported big-endian architecture checks"
    echo "hardware_status=community real-hardware reports requested for s390x, powerpc64, and big-endian AArch64"
} >"$evidence_dir/report.txt"

echo "big-endian QEMU checks: wrote $evidence_dir/report.txt"
echo "big-endian QEMU checks: ok"
