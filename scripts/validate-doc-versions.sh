#!/usr/bin/env sh
set -eu

cargo_version="$(
    sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"

if [ -z "$cargo_version" ]; then
    echo "doc versions: Cargo.toml package version is missing" >&2
    exit 1
fi

require_text() {
    file="$1"
    text="$2"
    if ! grep -F -q -- "$text" "$file"; then
        echo "doc versions: $file is missing required text: $text" >&2
        exit 1
    fi
}

reject_text() {
    file="$1"
    text="$2"
    if grep -F -q -- "$text" "$file"; then
        echo "doc versions: $file contains rejected text: $text" >&2
        exit 1
    fi
}

require_text CHANGELOG.md "## $cargo_version "

case "$cargo_version" in
    *-*)
        require_text CHANGELOG.md "## $cargo_version - Unreleased"
        require_text README.md "The development branch is"
        require_text README.md "\`$cargo_version\`"
        require_text docs/SIMD.md "$cargo_version"
        require_text docs/SIMD_ADMISSION.md "$cargo_version"
        ;;
    *)
        require_text README.md "The current public release is \`$cargo_version\`."
        require_text README.md "base64-ng = \"$cargo_version\""
        if [ "$cargo_version" = "1.1.0" ]; then
            require_text docs/SIMD_ADMISSION.md "Release status: \`1.1.x\`"
        else
            require_text docs/SIMD_ADMISSION.md "Release status: \`$cargo_version\`"
        fi
        reject_text README.md "-alpha"
        reject_text docs/SIMD.md "-alpha"
        reject_text docs/SIMD_ADMISSION.md "-alpha"
        ;;
esac

echo "doc versions: ok"
