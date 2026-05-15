#!/usr/bin/env sh
set -eu

if ! rustup run nightly cargo miri --version >/dev/null 2>&1; then
    echo "Miri checks: skipping; nightly Miri is not installed"
    exit 0
fi

evidence_dir="target/release-evidence/miri"
no_default_output="$evidence_dir/no-default-features.txt"
all_features_output="$evidence_dir/all-features.txt"
manifest="$evidence_dir/MANIFEST.txt"

mkdir -p "$evidence_dir"

echo "Miri checks: no-default-features scalar surface"
no_default_status=0
rustup run nightly cargo miri test --no-default-features >"$no_default_output" 2>&1 || no_default_status="$?"
cat "$no_default_output"

if [ "$no_default_status" -ne 0 ]; then
    all_features_status=99
else
    echo "Miri checks: all-features scalar, alloc, and stream surface"
    all_features_status=0
    rustup run nightly cargo miri test --all-features >"$all_features_output" 2>&1 || all_features_status="$?"
    cat "$all_features_output"
fi

{
    echo "base64-ng Miri evidence"
    echo
    echo "rustc:"
    rustup run nightly rustc -Vv
    echo
    echo "cargo:"
    rustup run nightly cargo -V
    echo
    echo "miri:"
    rustup run nightly cargo miri --version
    echo
    echo "system:"
    if command -v uname >/dev/null 2>&1; then
        uname -a
    else
        echo "uname unavailable"
    fi
    echo
    echo "commands:"
    echo "rustup run nightly cargo miri test --no-default-features"
    echo "rustup run nightly cargo miri test --all-features"
    echo
    echo "status:"
    echo "no_default_features=$no_default_status"
    echo "all_features=$all_features_status"
    echo
    echo "artifacts:"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$no_default_output" "$all_features_output" 2>/dev/null || true
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$no_default_output" "$all_features_output" 2>/dev/null || true
    else
        cksum "$no_default_output" "$all_features_output" 2>/dev/null || true
    fi
    echo
    echo "interpretation:"
    echo "This evidence records Miri coverage for scalar, alloc, and stream surfaces on this machine."
    echo "It checks undefined behavior that Miri can observe, but it is not a formal proof."
} >"$manifest"

echo "Miri checks: wrote $evidence_dir"

if [ "$no_default_status" -ne 0 ]; then
    exit "$no_default_status"
fi

if [ "$all_features_status" -ne 0 ]; then
    exit "$all_features_status"
fi

echo "Miri checks: ok"
