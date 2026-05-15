#!/usr/bin/env sh
set -eu

test -s docs/SIMD.md
test -s docs/SIMD_ADMISSION.md
test -s docs/UNSAFE.md
test -s docs/BENCHMARKS.md
test -s docs/RELEASE_EVIDENCE.md

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
    ' src/simd.rs
)"

if [ "$active_variants" != "Scalar" ]; then
    echo "simd admission: ActiveBackend must remain scalar-only until a backend is admitted" >&2
    printf '%s\n' "$active_variants" >&2
    exit 1
fi

if grep -R \
    -e 'ActiveBackend::Avx' \
    -e 'ActiveBackend::Neon' \
    -e 'ActiveBackend::Sse' \
    -e 'ActiveBackend::Wasm' \
    -e 'ActiveBackend::Simd' \
    src
then
    echo "simd admission: accelerated ActiveBackend dispatch was added without admission gate update" >&2
    exit 1
fi

if ! awk '
    /pub\(crate\) fn active_backend\(\) -> ActiveBackend/ {
        inside = 1
    }
    inside && /ActiveBackend::Scalar/ {
        found = 1
    }
    inside && /^}/ {
        exit found ? 0 : 1
    }
    END {
        if (!inside) {
            exit 1
        }
    }
' src/simd.rs; then
    echo "simd admission: active_backend must explicitly return ActiveBackend::Scalar" >&2
    exit 1
fi

for required_text in \
    "Do not advertise SIMD acceleration" \
    "Benchmark evidence that reports hardware" \
    "register-retention cleanup strategy" \
    "Performance numbers are release notes evidence only" \
    "Admitted backends: none" \
    "Active backend: scalar only" \
    "Do not advertise SIMD acceleration until this manifest names an admitted"
do
    if ! grep -R -q "$required_text" docs/SIMD.md docs/SIMD_ADMISSION.md docs/UNSAFE.md docs/RELEASE_EVIDENCE.md; then
        echo "simd admission: missing required SIMD admission text: $required_text" >&2
        exit 1
    fi
done

echo "simd admission: scalar-only dispatch gate ok"
