#!/usr/bin/env sh
set -eu

if ! grep -q '^#!\[deny(unsafe_code)\]' src/lib.rs; then
    echo "unsafe boundary: src/lib.rs must keep #![deny(unsafe_code)]"
    exit 1
fi

simd_allowed='src/simd.rs'
root_allowed='src/lib.rs'
matches="$(grep -RIl 'allow(unsafe_code)' src | sort || true)"
allowed="$(printf '%s\n%s' "$root_allowed" "$simd_allowed" | sort)"

if [ "$matches" != "$allowed" ]; then
    echo "unsafe boundary: allow(unsafe_code) may appear only in $root_allowed and $simd_allowed"
    if [ -n "$matches" ]; then
        echo "$matches"
    fi
    exit 1
fi

root_allow_count="$(grep -c '^#\[allow(unsafe_code)\]$' "$root_allowed" || true)"
if [ "$root_allow_count" -ne 7 ]; then
    echo "unsafe boundary: src/lib.rs must have exactly seven reviewed allow(unsafe_code) helpers"
    exit 1
fi

if ! awk '
    /^#\[allow\(unsafe_code\)\]$/ {
        allow_line = NR
    }
    /^fn wipe_bytes\(/ || /^fn wipe_barrier\(/ || /^fn wipe_vec_spare_capacity\(/ || /^fn ct_error_gate_barrier\(/ || /^fn constant_time_eq_same_len\(/ || /^fn string_from_validated_secret_bytes\(/ || /^fn ct_decode_alphabet_byte/ {
        if (allow_line != NR - 1) {
            failed = 1
        }
        seen += 1
    }
    END { exit failed || seen != 7 }
' "$root_allowed"; then
    echo "unsafe boundary: src/lib.rs allow(unsafe_code) must apply only to reviewed cleanup, secret-conversion, comparison, CT scan, and CT gate helpers"
    exit 1
fi

arch_matches="$(grep -RIl -e 'core::arch' -e 'std::arch' -e 'is_x86_feature_detected!' -e 'target_feature' src | sort || true)"
arch_allowed="$(printf '%s\n%s' "$root_allowed" "$simd_allowed" | sort)"

if [ "$arch_matches" != "$arch_allowed" ]; then
    echo "unsafe boundary: architecture intrinsics may appear only in $root_allowed cleanup barriers and $simd_allowed"
    if [ -n "$arch_matches" ]; then
        echo "$arch_matches"
    fi
    exit 1
fi

if ! grep -q 'core::arch::asm!' "$root_allowed"; then
    echo "unsafe boundary: src/lib.rs cleanup barrier must use the reviewed inline assembly barrier"
    exit 1
fi

if [ ! -s docs/UNSAFE.md ]; then
    echo "unsafe boundary: docs/UNSAFE.md must document unsafe sites"
    exit 1
fi

unsafe_functions="$(sed -n 's/^[[:space:]]*unsafe[[:space:]]*fn[[:space:]]*\([A-Za-z0-9_][A-Za-z0-9_]*\).*/\1/p' "$simd_allowed")"

if [ -z "$unsafe_functions" ]; then
    echo "unsafe boundary: expected documented prototype unsafe functions in $allowed"
    exit 1
fi

for symbol in $unsafe_functions; do
    if ! grep -q "$symbol" docs/UNSAFE.md; then
        echo "unsafe boundary: docs/UNSAFE.md must document $symbol"
        exit 1
    fi
done

if ! awk '
    /^[[:space:]]*unsafe[[:space:]]*\{/ {
        if (prev1 !~ /SAFETY:/ && prev2 !~ /SAFETY:/ && prev3 !~ /SAFETY:/ && prev4 !~ /SAFETY:/) {
            print FILENAME ":" FNR ": unsafe block is missing a nearby SAFETY explanation"
            failed = 1
        }
    }
    {
        prev4 = prev3
        prev3 = prev2
        prev2 = prev1
        prev1 = $0
    }
    END { exit failed }
' "$root_allowed" "$simd_allowed"; then
    echo "unsafe boundary: every unsafe block must have a nearby SAFETY explanation"
    exit 1
fi

echo "unsafe boundary: ok"
