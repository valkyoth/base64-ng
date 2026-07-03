#!/usr/bin/env sh
set -eu

test -s docs/SIMD.md
test -s docs/SIMD_ADMISSION.md
test -s docs/UNSAFE.md
test -s docs/BENCHMARKS.md
test -s docs/RELEASE_EVIDENCE.md
test -s docs/SIMD_ENCODE_ADMISSION_DRAFT.md

active_variants="$(
    awk '
        /pub\(crate\) enum ActiveBackend/ {
            inside = 1
            next
        }
        inside && /^}/ {
            inside = 0
        }
        inside {
            line = $0
            sub(/\/\/.*/, "", line)
            if (line ~ /^[[:space:]]*[A-Za-z][A-Za-z0-9_]*,/) {
                gsub(/[[:space:],]/, "", line)
                print line
            }
        }
    ' src/simd/mod.rs
)"

expected_active_variants="Scalar
Avx512Vbmi
Avx2
Ssse3Sse41
Neon
WasmSimd128"
if [ "$active_variants" != "$expected_active_variants" ]; then
    echo "simd admission: ActiveBackend must contain only Scalar and admitted AVX-512/AVX2/SSSE3/NEON/wasm encode" >&2
    printf '%s\n' "$active_variants" >&2
    exit 1
fi

if grep -R -e 'ActiveBackend::Simd' src; then
    echo "simd admission: non-admitted accelerated ActiveBackend dispatch was added without admission gate update" >&2
    exit 1
fi

if ! awk '
    /fn detect_active_backend\(\) -> ActiveBackend/ {
        inside = 1
    }
    inside && /ActiveBackend::Avx512Vbmi/ {
        avx512 = 1
    }
    inside && /ActiveBackend::Avx2/ {
        avx2 = 1
    }
    inside && /ActiveBackend::Ssse3Sse41/ {
        ssse3 = 1
    }
    inside && /ActiveBackend::Neon/ {
        neon = 1
    }
    inside && /ActiveBackend::WasmSimd128/ {
        wasm = 1
    }
    inside && /ActiveBackend::Scalar/ {
        scalar = 1
    }
    inside && /^}/ {
        exit (scalar && avx512 && avx2 && ssse3 && neon && wasm) ? 0 : 1
    }
    END {
        if (!inside) {
            exit 1
        }
    }
' src/simd/mod.rs; then
    echo "simd admission: detect_active_backend must explicitly return admitted AVX-512, AVX2, SSSE3/SSE4.1, NEON, wasm simd128, and scalar fallback" >&2
    exit 1
fi

for required_text in \
    "Do not advertise SIMD acceleration" \
    "Benchmark evidence that reports hardware" \
    "register-retention cleanup strategy" \
    "explicit register cleanup implementation and tests" \
    "real non-dispatchable prototype" \
    "candidate only" \
    "admitted backend" \
    "std x86/x86_64 and little-endian std aarch64 dispatch only" \
    "Decode acceleration" \
    "Required precision" \
    "Performance numbers are release notes evidence only" \
    "Admitted backends: AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode, NEON encode, AVX-512 VBMI strict decode, AVX2 strict decode, SSSE3/SSE4.1 strict decode, and NEON strict decode" \
    "Active backend priority: AVX-512 VBMI, then AVX2, then SSSE3/SSE4.1 on x86/x86_64; NEON on little-endian aarch64" \
    "wasm simd128 runtime smoke evidence" \
    "The active non-scalar backends" \
    "Advertise SIMD acceleration only with the admitted backend name and scope"
do
    if ! grep -R -q "$required_text" docs/SIMD.md docs/SIMD_ADMISSION.md docs/UNSAFE.md docs/RELEASE_EVIDENCE.md docs/SIMD_ENCODE_ADMISSION_DRAFT.md; then
        echo "simd admission: missing required SIMD admission text: $required_text" >&2
        exit 1
    fi
done

backend_rows="$(
    awk '
        /^\| AVX-512 VBMI / || /^\| AVX2 / || /^\| SSSE3\/SSE4\.1 / || /^\| NEON / || /^\| wasm `simd128` / {
            print
        }
    ' docs/SIMD_ADMISSION.md
)"

backend_row_count="$(printf '%s\n' "$backend_rows" | sed '/^$/d' | wc -l | tr -d ' ')"
if [ "$backend_row_count" -ne 5 ]; then
    echo "simd admission: expected exactly five backend rows in docs/SIMD_ADMISSION.md" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

avx512_row="$(printf '%s\n' "$backend_rows" | grep '^| AVX-512 VBMI ')"
if ! printf '%s\n' "$avx512_row" | grep '| admitted backend |' >/dev/null 2>&1; then
    echo "simd admission: AVX-512 VBMI row must be an admitted backend" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

avx2_row="$(printf '%s\n' "$backend_rows" | grep '^| AVX2 ')"
if ! printf '%s\n' "$avx2_row" | grep '| admitted backend |' >/dev/null 2>&1; then
    echo "simd admission: AVX2 row must be an admitted backend" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

ssse3_row="$(printf '%s\n' "$backend_rows" | grep '^| SSSE3/SSE4\.1 ')"
if ! printf '%s\n' "$ssse3_row" | grep '| admitted backend |' >/dev/null 2>&1; then
    echo "simd admission: SSSE3/SSE4.1 row must be an admitted backend" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

neon_row="$(printf '%s\n' "$backend_rows" | grep '^| NEON ')"
if ! printf '%s\n' "$neon_row" | grep '| admitted backend |' >/dev/null 2>&1; then
    echo "simd admission: NEON row must be an admitted backend" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

wasm_row="$(printf '%s\n' "$backend_rows" | grep '^| wasm `simd128` ')"
if ! printf '%s\n' "$wasm_row" | grep '| admitted backend |' >/dev/null 2>&1; then
    echo "simd admission: wasm simd128 row must be an admitted backend" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

if ! grep -q 'Node/V8, Wasmtime, and Chromium-family browser runtime smoke evidence' docs/SIMD_ADMISSION.md docs/SIMD.md; then
    echo "simd admission: wasm admission must name Node/V8, Wasmtime, and Chromium-family browser runtime smoke evidence" >&2
    exit 1
fi

if ! grep -q 'Chromium-family browser runtime smoke evidence' docs/SIMD_ADMISSION.md docs/SIMD.md; then
    echo "simd admission: wasm admission must name Chromium-family browser runtime smoke evidence" >&2
    exit 1
fi

echo "simd admission: AVX-512 VBMI, AVX2, SSSE3/SSE4.1, NEON, and wasm simd128 encode/decode admission gate ok"
