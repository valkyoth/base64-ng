#!/usr/bin/env sh
set -eu

if ! rustup run nightly cargo miri --version >/dev/null 2>&1; then
    echo "Miri checks: skipping; nightly Miri is not installed"
    exit 0
fi

echo "Miri checks: no-default-features scalar surface"
rustup run nightly cargo miri test --no-default-features

echo "Miri checks: all-features scalar, alloc, and stream surface"
rustup run nightly cargo miri test --all-features

echo "Miri checks: ok"
