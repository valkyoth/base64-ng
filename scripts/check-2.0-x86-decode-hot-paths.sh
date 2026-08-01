#!/usr/bin/env sh
set -eu

wrapper_file="src/simd/x86/decode.rs"
kernel_file="src/simd/x86/decode_direct.rs"
token_file="src/simd/static_token.rs"

for required in \
    'decode_full_blocks_ssse3_sse41' \
    'decode_full_blocks_avx2' \
    'decode_full_blocks_avx512' \
    'decode_16_bytes_ssse3_sse41' \
    'decode_32_bytes_avx2' \
    'decode_64_bytes_avx512' \
    'map_ascii_to_values_ssse3' \
    'map_ascii_to_values_avx2' \
    'map_ascii_to_values_avx512' \
    '_mm_maddubs_epi16' \
    '_mm256_maddubs_epi16' \
    '_mm512_maddubs_epi16' \
    '_mm_shuffle_epi8' \
    '_mm256_shuffle_epi8' \
    '_mm512_permutexvar_epi8' \
    '_mm512_mask_storeu_epi8' \
    '_mm_storel_epi64'
do
    if ! grep -F -q "$required" "$wrapper_file" "$kernel_file"; then
        echo "2.0 x86 decode: production hot path is missing: $required" >&2
        exit 1
    fi
done

for function in decode_16_bytes_ssse3_sse41 decode_32_bytes_avx2 decode_64_bytes_avx512; do
    body="$(sed -n "/unsafe fn $function/,/^}/p" "$kernel_file")"
    for forbidden in 'scalar::' fill_decode_values copy_verified_decode_output wipe_bytes; do
        if printf '%s\n' "$body" | grep -F -q "$forbidden"; then
            echo "2.0 x86 decode: $function retains prototype operation: $forbidden" >&2
            exit 1
        fi
    done
done

for required in decode_standard decode_url_safe decode_slice_avx512 decode_slice_avx2 decode_slice_ssse3_sse41 decode_backend::decode_checked; do
    if ! grep -F -q "$required" "$token_file"; then
        echo "2.0 x86 decode: static token contract is missing: $required" >&2
        exit 1
    fi
done

echo "2.0 x86 decode: exhaustive symbol, malformed-position, tail, and error evidence"
cargo test --all-features --lib 'simd::x86_decode_direct_tests'
scripts/test-x86-decode-performance.py

echo "2.0 x86 decode: forced-backend fuzz contract"
grep -F -q 'name = "x86_decode"' fuzz/Cargo.toml
grep -F -q 'compare_backend(Backend::Ssse3Sse41' fuzz/fuzz_targets/x86_decode.rs
grep -F -q 'compare_backend(Backend::Avx2' fuzz/fuzz_targets/x86_decode.rs
grep -F -q 'compare_backend(Backend::Avx512Vbmi' fuzz/fuzz_targets/x86_decode.rs
cargo check --manifest-path fuzz/Cargo.toml --bin x86_decode

case "$(rustc -vV | sed -n 's/^host: //p')" in
    x86_64-*|i686-*)
        smoke_dir="target/x86-static-decode-smoke"
        mkdir -p "$smoke_dir/src"
        cp portability/x86_static_encode_smoke/src/main.rs "$smoke_dir/src/main.rs"
        cat >"$smoke_dir/Cargo.toml" <<'MANIFEST'
[package]
name = "base64-ng-x86-static-decode-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[features]
checked-backend = ["base64-ng/checked-backend"]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["simd"] }
MANIFEST
        echo "2.0 x86 decode: no_std static SSSE3/SSE4.1 execution"
        RUSTFLAGS='-C target-feature=+ssse3,+sse4.1' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml"
        echo "2.0 x86 decode: no_std static AVX2 execution"
        RUSTFLAGS='-C target-feature=+avx2' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml"
        host_flags=""
        if [ -r /proc/cpuinfo ]; then
            host_flags="$(sed -n 's/^flags[[:space:]]*: / /p' /proc/cpuinfo | sed -n '1p')"
        fi
        case "$host_flags" in
            *' avx512f '*avx512bw*avx512vl*avx512vbmi*)
                echo "2.0 x86 decode: no_std static AVX-512 VBMI execution"
                RUSTFLAGS='-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi' \
                    cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml"
                ;;
            *)
                echo "2.0 x86 decode: skipping static AVX-512 execution; matching Linux hardware not detected"
                ;;
        esac
        echo "2.0 x86 decode: checked no_std static SSSE3/SSE4.1 execution"
        RUSTFLAGS='-C target-feature=+ssse3,+sse4.1' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml" \
            --features checked-backend
        echo "2.0 x86 decode: checked no_std static AVX2 execution"
        RUSTFLAGS='-C target-feature=+avx2' \
            cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml" \
            --features checked-backend
        case "$host_flags" in
            *' avx512f '*avx512bw*avx512vl*avx512vbmi*)
                echo "2.0 x86 decode: checked no_std static AVX-512 VBMI execution"
                RUSTFLAGS='-C target-feature=+avx512f,+avx512bw,+avx512vl,+avx512vbmi' \
                    cargo run --quiet --offline --manifest-path "$smoke_dir/Cargo.toml" \
                    --features checked-backend
                ;;
        esac
        ;;
esac

if [ "${BASE64_NG_RUN_COMMIT28_PERF:-${BASE64_NG_RUN_COMMIT27_PERF:-0}}" = "1" ]; then
    evidence="${BASE64_NG_COMMIT28_PERF_FILE:-target/release-evidence/commit-28-x86-decode.csv}"
    mkdir -p "$(dirname "$evidence")"
    echo "2.0 x86 decode: focused exact-backend performance campaign"
    RUSTFLAGS='--cfg base64_ng_perf_evidence' \
        cargo run --quiet --release --manifest-path perf/Cargo.toml -- x86-decode >"$evidence"
    scripts/validate-x86-decode-performance.py "$evidence"
else
    echo "2.0 x86 decode: performance run skipped; set BASE64_NG_RUN_COMMIT28_PERF=1"
fi

scripts/validate-unsafe-boundary.sh
echo "2.0 x86 decode: direct production kernels and static contract ok"
