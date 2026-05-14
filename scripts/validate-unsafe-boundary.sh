#!/usr/bin/env sh
set -eu

if ! grep -q '^#!\[deny(unsafe_code)\]' src/lib.rs; then
    echo "unsafe boundary: src/lib.rs must keep #![deny(unsafe_code)]"
    exit 1
fi

allowed='src/simd.rs'
matches="$(grep -RIl 'allow(unsafe_code)' src || true)"

if [ "$matches" != "$allowed" ]; then
    echo "unsafe boundary: allow(unsafe_code) may appear only in $allowed"
    if [ -n "$matches" ]; then
        echo "$matches"
    fi
    exit 1
fi

arch_matches="$(grep -RIl -e 'core::arch' -e 'std::arch' -e 'is_x86_feature_detected!' -e 'target_feature' src || true)"

if [ "$arch_matches" != "$allowed" ]; then
    echo "unsafe boundary: architecture intrinsics and target-feature gates may appear only in $allowed"
    if [ -n "$arch_matches" ]; then
        echo "$arch_matches"
    fi
    exit 1
fi

if [ ! -s docs/UNSAFE.md ]; then
    echo "unsafe boundary: docs/UNSAFE.md must document unsafe sites"
    exit 1
fi

unsafe_functions="$(sed -n 's/^[[:space:]]*pub(super)[[:space:]]*unsafe[[:space:]]*fn[[:space:]]*\([A-Za-z0-9_][A-Za-z0-9_]*\).*/\1/p' "$allowed")"

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
' "$allowed"; then
    echo "unsafe boundary: every unsafe block must have a nearby SAFETY explanation"
    exit 1
fi

echo "unsafe boundary: ok"
