#!/usr/bin/env sh
set -eu

wrapper_file="src/simd/neon.rs"
kernel_file="src/simd/neon/direct.rs"
token_file="src/simd/static_token.rs"
host="$(rustc -vV | sed -n 's/^host: //p')"
case "$host" in
    aarch64-*) neon_target="$host" ;;
    *) neon_target="aarch64-unknown-linux-gnu" ;;
esac

for required in \
    'encode_full_blocks_neon' \
    'decode_full_blocks_neon' \
    'direct::encode_12_bytes' \
    'direct::decode_16_bytes' \
    'vld1_u8(input.as_ptr())' \
    'core::ptr::read_unaligned' \
    'vminvq_u8(valid)' \
    'map_ascii_to_values' \
    'vqtbl1q_u8' \
    'vst1_u8(output.as_mut_ptr()' \
    'core::ptr::write_unaligned' \
    'clear_neon_registers_after_vector_block'
do
    if ! grep -F -q "$required" "$wrapper_file" "$kernel_file"; then
        echo "2.0 NEON: production hot path is missing: $required" >&2
        exit 1
    fi
done

for function in encode_12_bytes decode_16_bytes; do
    body="$(sed -n "/unsafe fn $function/,/^}/p" "$kernel_file")"
    for forbidden in 'scalar::' fill_decode_values copy_verified_decode_output wipe_bytes staged; do
        if printf '%s\n' "$body" | grep -F -q "$forbidden"; then
            echo "2.0 NEON: direct $function retains prototype operation: $forbidden" >&2
            exit 1
        fi
    done
done

for required in 'Backend::Neon' 'encode_slice_neon' 'decode_slice_neon' \
    'encode_backend::encode_checked' 'decode_backend::decode_checked'; do
    if ! grep -F -q "$required" "$token_file"; then
        echo "2.0 NEON: static token contract is missing: $required" >&2
        exit 1
    fi
done

echo "2.0 NEON: cross-target compile and lint evidence"
cargo check --target "$neon_target" --all-features --all-targets
cargo check --target "$neon_target" --no-default-features \
    --features simd,checked-backend --lib
cargo clippy --target "$neon_target" --all-features --all-targets -- -D warnings

echo "2.0 NEON: fuzz and performance policy evidence"
grep -F -q 'name = "neon"' fuzz/Cargo.toml
grep -F -q 'StaticBackendToken::assume_supported(Backend::Neon)' fuzz/fuzz_targets/neon.rs
cargo check --manifest-path fuzz/Cargo.toml --bin neon
scripts/test-neon-performance.py

case "$host" in
    aarch64-*)
        echo "2.0 NEON: exhaustive direct and public-surface tests on real AArch64"
        cargo test --all-features --lib 'simd::neon_direct_tests'
        cargo test --all-features --lib 'simd::neon_decode_tests'
        cargo test --all-features --lib 'simd::tests::neon_encode_block'

        smoke_dir="target/aarch64-static-neon-smoke"
        mkdir -p "$smoke_dir/src"
        cp portability/aarch64_static_neon_smoke/src/main.rs "$smoke_dir/src/main.rs"
        cat >"$smoke_dir/Cargo.toml" <<'MANIFEST'
[package]
name = "base64-ng-aarch64-static-neon-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[features]
checked-backend = ["base64-ng/checked-backend"]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["simd"] }
MANIFEST
        echo "2.0 NEON: static no_std execution"
        RUSTFLAGS='-C target-feature=+neon' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml"
        echo "2.0 NEON: checked static no_std execution"
        RUSTFLAGS='-C target-feature=+neon' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml" \
            --features checked-backend
        ;;
    *)
        echo "2.0 NEON: real-device execution deferred to check_macos.sh/check_aarch64_linux.sh"
        ;;
esac

if [ "${BASE64_NG_RUN_COMMIT29_PERF:-0}" = "1" ]; then
    evidence="${BASE64_NG_COMMIT29_PERF_FILE:-target/release-evidence/commit-29-neon.csv}"
    mkdir -p "$(dirname "$evidence")"
    echo "2.0 NEON: exact-backend performance campaign"
    RUSTFLAGS='--cfg base64_ng_perf_evidence' \
        cargo run --quiet --release --manifest-path perf/Cargo.toml -- neon >"$evidence"
    scripts/validate-neon-performance.py "$evidence"
else
    echo "2.0 NEON: performance run skipped; set BASE64_NG_RUN_COMMIT29_PERF=1 on AArch64"
fi

BASE64_NG_ALLOW_DIRTY_EVIDENCE=1 BASE64_NG_NEON_ASM_TARGET="$neon_target" \
    scripts/generate_neon_asm_evidence.sh
scripts/validate-unsafe-boundary.sh
echo "2.0 NEON: direct kernels, static contract, and evidence gates ok"
