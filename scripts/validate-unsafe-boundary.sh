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

echo "unsafe boundary: ok"
