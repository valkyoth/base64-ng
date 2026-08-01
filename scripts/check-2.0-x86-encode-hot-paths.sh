#!/usr/bin/env sh
set -eu

source_file="src/simd/x86/mod.rs"
token_file="src/simd/static_token.rs"

for required in \
    'encode_24_bytes_avx2_inner' \
    '_mm_loadu_si128(input.as_ptr()' \
    '_mm_loadl_epi64(input.as_ptr().add(16)' \
    'encode_full_blocks_avx2' \
    'encode_12_bytes_ssse3_sse41_inner' \
    'core::ptr::read_unaligned(input.as_ptr().add(8)' \
    'encode_full_blocks_ssse3_sse41' \
    '_mm256_shuffle_epi8(lookup, lookup_index)' \
    '_mm_shuffle_epi8(lookup, lookup_index)'
do
    if ! grep -F -q "$required" "$source_file"; then
        echo "2.0 x86 encode: production hot path is missing: $required" >&2
        exit 1
    fi
done

for function in encode_24_bytes_avx2_inner encode_12_bytes_ssse3_sse41_inner; do
    body="$(sed -n "/unsafe fn $function/,/^}/p" "$source_file")"
    for forbidden in staged wipe_bytes clear_xmm_registers clear_ymm_registers; do
        if printf '%s\n' "$body" | grep -F -q "$forbidden"; then
            echo "2.0 x86 encode: $function retains prototype operation: $forbidden" >&2
            exit 1
        fi
    done
done

for required in 'encode_standard' 'encode_url_safe' 'encode_slice_avx2' 'encode_slice_ssse3_sse41' 'encode_backend::encode_checked'; do
    if ! grep -F -q "$required" "$token_file"; then
        echo "2.0 x86 encode: static token contract is missing: $required" >&2
        exit 1
    fi
done

echo "2.0 x86 encode: exhaustive block, tail, and static-token evidence"
cargo test --all-features --lib 'simd::x86_encode_tests'
scripts/test-x86-encode-performance.py

echo "2.0 x86 encode: forced-backend fuzz and Miri wrapper contracts"
grep -F -q 'name = "x86_encode"' fuzz/Cargo.toml
grep -F -q 'compare_backend(Backend::Ssse3Sse41' fuzz/fuzz_targets/x86_encode.rs
grep -F -q 'compare_backend(Backend::Avx2' fuzz/fuzz_targets/x86_encode.rs
grep -F -q 'tests::encode_backend_boundary_uses_only_admitted_backends' scripts/check_miri.sh
cargo check --manifest-path fuzz/Cargo.toml --bin x86_encode

case "$(rustc -vV | sed -n 's/^host: //p')" in
    x86_64-*|i686-*)
        smoke_dir="target/x86-static-encode-smoke"
        mkdir -p "$smoke_dir/src"
        cp portability/x86_static_encode_smoke/src/main.rs "$smoke_dir/src/main.rs"
        cat >"$smoke_dir/Cargo.toml" <<'MANIFEST'
[package]
name = "base64-ng-x86-static-encode-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[features]
checked-backend = ["base64-ng/checked-backend"]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["simd"] }
MANIFEST
        echo "2.0 x86 encode: no_std static SSSE3/SSE4.1 execution"
        RUSTFLAGS='-C target-feature=+ssse3,+sse4.1' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml"
        echo "2.0 x86 encode: no_std static AVX2 execution"
        RUSTFLAGS='-C target-feature=+avx2' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml"
        echo "2.0 x86 encode: checked no_std static SSSE3/SSE4.1 execution"
        RUSTFLAGS='-C target-feature=+ssse3,+sse4.1' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml" \
            --features checked-backend
        echo "2.0 x86 encode: checked no_std static AVX2 execution"
        RUSTFLAGS='-C target-feature=+avx2' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml" \
            --features checked-backend
        ;;
esac

if [ "${BASE64_NG_RUN_COMMIT25_PERF:-0}" = "1" ]; then
    evidence="${BASE64_NG_COMMIT25_PERF_FILE:-target/release-evidence/commit-25-x86-encode.csv}"
    mkdir -p "$(dirname "$evidence")"
    echo "2.0 x86 encode: focused exact-backend performance campaign"
    RUSTFLAGS='--cfg base64_ng_perf_evidence' \
        cargo run --quiet --release --manifest-path perf/Cargo.toml -- x86-encode >"$evidence"
    scripts/validate-x86-encode-performance.py "$evidence"
else
    echo "2.0 x86 encode: performance run skipped; set BASE64_NG_RUN_COMMIT25_PERF=1"
fi

scripts/validate-unsafe-boundary.sh
echo "2.0 x86 encode: direct production kernels and static contract ok"
