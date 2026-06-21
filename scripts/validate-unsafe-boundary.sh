#!/usr/bin/env sh
set -eu

if ! grep -q '^#!\[deny(unsafe_code)\]' src/lib.rs; then
    echo "unsafe boundary: src/lib.rs must keep #![deny(unsafe_code)]"
    exit 1
fi

simd_allow_files='src/simd/mod.rs src/simd/x86/cleanup.rs src/simd/x86/mod.rs'
simd_boundary_files='src/simd/mod.rs src/simd/wasm.rs src/simd/x86/cleanup.rs src/simd/x86/mod.rs'
simd_tests_allowed='src/simd/tests.rs'
root_allowed='src/lib.rs'
cleanup_allowed='src/cleanup.rs'
ct_allowed_files='src/ct/decode.rs src/ct/equality.rs'
matches="$(grep -RIl 'allow(unsafe_code)' src | sort || true)"
allowed="$(printf '%s\n' "$cleanup_allowed" src/ct/decode.rs src/ct/equality.rs $simd_allow_files | sort)"

if [ "$matches" != "$allowed" ]; then
    echo "unsafe boundary: allow(unsafe_code) may appear only in $cleanup_allowed, src/ct/decode.rs, src/ct/equality.rs, and src/simd/"
    if [ -n "$matches" ]; then
        echo "$matches"
    fi
    exit 1
fi

root_allow_count="$(grep -c '^#\[allow(unsafe_code)\]$' "$root_allowed" || true)"
if [ "$root_allow_count" -ne 0 ]; then
    echo "unsafe boundary: src/lib.rs must not carry module-local allow(unsafe_code) helpers"
    exit 1
fi

cleanup_allow_count="$(grep -c '^#\[allow(unsafe_code)\]$' "$cleanup_allowed" || true)"
if [ "$cleanup_allow_count" -ne 3 ]; then
    echo "unsafe boundary: src/cleanup.rs must have exactly three reviewed allow(unsafe_code) helpers"
    exit 1
fi

ct_allow_count="$(grep -h -c '^#\[allow(unsafe_code)\]$' $ct_allowed_files | awk '{ total += $1 } END { print total + 0 }')"
if [ "$ct_allow_count" -ne 4 ]; then
    echo "unsafe boundary: src/ct/ must have exactly four reviewed allow(unsafe_code) helpers"
    exit 1
fi

if ! awk '
    /^#\[allow\(unsafe_code\)\]$/ {
        allow_line = NR
    }
    /^(pub\((crate|super)\) )?fn wipe_bytes\(/ || /^fn wipe_barrier\(/ || /^(pub\((crate|super)\) )?fn wipe_vec_spare_capacity\(/ || /^(pub\((crate|super)\) )?fn ct_error_gate_barrier\(/ || /^fn constant_time_eq_same_len\(/ || /^(pub\((crate|super)\) )?fn ct_accumulate_u8\(/ || /^(pub\((crate|super)\) )?fn ct_decode_alphabet_byte/ {
        if (allow_line != NR - 1) {
            failed = 1
        }
        seen += 1
    }
    END { exit failed || seen != 7 }
' "$cleanup_allowed" $ct_allowed_files; then
    echo "unsafe boundary: allow(unsafe_code) must apply only to reviewed cleanup, comparison, CT accumulator, CT scan, and CT gate helpers"
    exit 1
fi

arch_matches="$(grep -RIl -e 'core::arch' -e 'std::arch' -e 'is_x86_feature_detected!' -e 'target_feature' src | sort || true)"
arch_allowed="$(printf '%s\n' src/ct/equality.rs "$cleanup_allowed" $simd_boundary_files "$simd_tests_allowed" | sort)"

if [ "$arch_matches" != "$arch_allowed" ]; then
    echo "unsafe boundary: architecture intrinsics may appear only in src/ct/equality.rs CT barriers, $cleanup_allowed cleanup barriers, and src/simd/"
    if [ -n "$arch_matches" ]; then
        echo "$arch_matches"
    fi
    exit 1
fi

if ! grep -q 'core::arch::asm!' "$cleanup_allowed"; then
    echo "unsafe boundary: src/cleanup.rs cleanup barrier must use the reviewed inline assembly barrier"
    exit 1
fi

if [ ! -s docs/UNSAFE.md ]; then
    echo "unsafe boundary: docs/UNSAFE.md must document unsafe sites"
    exit 1
fi

unsafe_functions="$(sed -n 's/^[[:space:]]*\(\(pub(\(crate\|super\))[[:space:]]*\)\?\)unsafe[[:space:]]*fn[[:space:]]*\([A-Za-z0-9_][A-Za-z0-9_]*\).*/\4/p' $simd_boundary_files)"

if [ -z "$unsafe_functions" ]; then
    echo "unsafe boundary: expected documented prototype unsafe functions in src/simd/"
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
' $ct_allowed_files "$cleanup_allowed" $simd_boundary_files; then
    echo "unsafe boundary: every unsafe block must have a nearby SAFETY explanation"
    exit 1
fi

echo "unsafe boundary: ok"
