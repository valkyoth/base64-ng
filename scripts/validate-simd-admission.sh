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
Avx2
Ssse3Sse41"
if [ "$active_variants" != "$expected_active_variants" ]; then
    echo "simd admission: ActiveBackend must contain only Scalar and admitted AVX2/SSSE3 encode" >&2
    printf '%s\n' "$active_variants" >&2
    exit 1
fi

if grep -R \
    -e 'ActiveBackend::Avx512' \
    -e 'ActiveBackend::Neon' \
    -e 'ActiveBackend::Wasm' \
    -e 'ActiveBackend::Simd' \
    src
then
    echo "simd admission: non-admitted accelerated ActiveBackend dispatch was added without admission gate update" >&2
    exit 1
fi

if ! awk '
    /pub\(crate\) fn active_backend\(\) -> ActiveBackend/ {
        inside = 1
    }
    inside && /ActiveBackend::Avx2/ {
        avx2 = 1
    }
    inside && /ActiveBackend::Ssse3Sse41/ {
        ssse3 = 1
    }
    inside && /ActiveBackend::Scalar/ {
        scalar = 1
    }
    inside && /^}/ {
        exit (scalar && avx2 && ssse3) ? 0 : 1
    }
    END {
        if (!inside) {
            exit 1
        }
    }
' src/simd/mod.rs; then
    echo "simd admission: active_backend must explicitly return admitted AVX2, SSSE3/SSE4.1, and scalar fallback" >&2
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
    "x86/x86_64 runtime dispatch only" \
    "Decode acceleration" \
    "Required precision" \
    "Performance numbers are release notes evidence only" \
    "Admitted backends: AVX2 encode and SSSE3/SSE4.1 encode" \
    "Active backend priority: AVX2, then SSSE3/SSE4.1" \
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

non_admitted_rows="$(printf '%s\n' "$backend_rows" | grep -v '^| AVX2 ' | grep -v '^| SSSE3/SSE4\.1 ')"
if printf '%s\n' "$non_admitted_rows" | grep -v '| real non-dispatchable prototype |' >/dev/null 2>&1; then
    echo "simd admission: non-admitted backend rows must remain real non-dispatchable prototypes" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

if printf '%s\n' "$backend_rows" | grep 'real fixed-block encode prototype' | grep -v 'non-dispatchable' >/dev/null 2>&1; then
    echo "simd admission: real prototype rows must say non-dispatchable" >&2
    printf '%s\n' "$backend_rows" >&2
    exit 1
fi

if grep -q 'AVX-512 .*admitted backend\|NEON .*admitted backend\|wasm .*admitted backend' docs/SIMD_ADMISSION.md docs/SIMD.md; then
    echo "simd admission: non-AVX2/SSSE3 admitted backend wording requires gate update" >&2
    exit 1
fi

echo "simd admission: AVX2 and SSSE3/SSE4.1 encode admission gate ok"
