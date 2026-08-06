#!/usr/bin/env sh
set -eu
ulimit -c 0

. scripts/evidence-source.sh
evidence_capture_source "SVE QEMU evidence"

script_revision="2026-08-06-signed-provenance-v4"
evidence_dir="target/release-evidence/sve-qemu"
target="aarch64-unknown-linux-musl"
target_key="AARCH64_UNKNOWN_LINUX_MUSL"

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "SVE QEMU checks: missing command: $1" >&2
        exit 1
    fi
}

if ! rustup target list --installed | grep -qx "$target"; then
    echo "SVE QEMU checks: missing Rust target: $target" >&2
    exit 1
fi

host="$(rustc -vV | sed -n 's/^host: //p')"
linker="$(rustc --print sysroot)/lib/rustlib/$host/bin/rust-lld"
if [ ! -x "$linker" ]; then
    echo "SVE QEMU checks: rust-lld is missing from the active toolchain" >&2
    exit 1
fi

runner_for_vq() {
    printf '%s' "qemu-aarch64 -cpu max,sve-max-vq=$1"
}

runner_for_fallback() {
    printf '%s' "qemu-aarch64 -cpu max,sve=off"
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
    env \
        RUSTFLAGS="${RUSTFLAGS:-} --cfg base64_ng_sve_candidate" \
        "BASE64_NG_TEST_SUBPROCESS_RUNNER=$runner" \
        "CARGO_TARGET_${target_key}_LINKER=$linker" \
        "CARGO_TARGET_${target_key}_RUNNER=$runner" \
        cargo "$@"
}

require_command qemu-aarch64
rustc --print cfg --target "$target" | grep -q 'target_arch="aarch64"'
rustc --print cfg --target "$target" | grep -q 'target_endian="little"'

runner_fallback="$(runner_for_fallback)"
runner_128="$(runner_for_vq 1)"
runner_256="$(runner_for_vq 2)"
runner_512="$(runner_for_vq 4)"

echo "SVE QEMU checks: script=$script_revision"
echo "SVE QEMU checks: host=$host"
echo "SVE QEMU checks: rustc=$(rustc --version)"
echo "SVE QEMU checks: cargo=$(cargo --version)"
echo "SVE QEMU checks: linker=$linker"

echo "SVE QEMU checks: scalar/NEON fallback default-feature suite"
cargo_target "$runner_fallback" test --target "$target" --lib --tests -- --test-threads=1

echo "SVE QEMU checks: scalar/NEON fallback all-feature suite"
cargo_target "$runner_fallback" test --target "$target" --all-features --lib --tests -- --test-threads=1

echo "SVE QEMU checks: scalar/NEON fallback no-default suite"
cargo_target "$runner_fallback" test --target "$target" --no-default-features --lib --tests -- --test-threads=1

echo "SVE QEMU checks: all-feature doctests"
cargo_target "$runner_fallback" test --target "$target" --all-features --doc -- --test-threads=1

echo "SVE QEMU checks: no-default doctests"
cargo_target "$runner_fallback" test --target "$target" --no-default-features --doc -- --test-threads=1

for vector in "128:$runner_128" "256:$runner_256" "512:$runner_512"; do
    bits="${vector%%:*}"
    runner="${vector#*:}"
    echo "SVE QEMU checks: SVE candidate vector length=$bits"
    cargo_candidate "$runner" test --target "$target" --all-features --lib sve_ -- --nocapture --test-threads=1
done

echo "SVE QEMU checks: no_std static +sve candidate compile"
env \
    RUSTFLAGS="${RUSTFLAGS:-} --cfg base64_ng_sve_candidate -C target-feature=+sve" \
    "CARGO_TARGET_${target_key}_LINKER=$linker" \
    cargo check --target "$target" --no-default-features --features simd --lib

evidence_verify_source "SVE QEMU evidence"

mkdir -p "$evidence_dir"
{
    echo "base64-ng AArch64 SVE QEMU evidence"
    echo "script=$script_revision"
    echo "source_commit=$EVIDENCE_SOURCE_COMMIT"
    echo "tree_state=$EVIDENCE_SOURCE_TREE_STATE"
    echo "result=pass"
    echo "target=$target"
    echo "linker=$linker"
    echo "qemu=$(qemu-aarch64 --version | sed -n '1p')"
    echo "rustc=$(rustc --version)"
    echo "cargo=$(cargo --version)"
    echo "fallback_cpu=max,sve=off"
    echo "candidate_vector_lengths=128,256,512"
    echo "candidate_isa=SVE"
    echo "qemu_test_threads=1"
    echo "candidate_dispatch=internal-QEMU-only"
    echo "public_dispatch=admitted-NEON-or-scalar"
    echo "evidence_scope=portable and NEON fallback coverage plus non-admitted SVE encode/decode candidate correctness"
    echo "not_evidence_for=real hardware correctness, performance, timing, ABI preservation, or production admission"
    echo "hardware_status=two real SVE systems with different vector lengths required before public dispatch admission"
} >"$evidence_dir/report.txt"

evidence_verify_source "SVE QEMU evidence"
echo "SVE QEMU checks: wrote $evidence_dir/report.txt"
echo "SVE QEMU checks: ok"
