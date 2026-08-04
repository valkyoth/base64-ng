#!/usr/bin/env sh
set -eu

cargo_version="$(
    sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
release_policy="$(
    sed -n 's/^policy = "\([^"]*\)"/\1/p' release-crates.toml | sed -n '1p'
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

if [ "$release_policy" = "development-blocked" ]; then
    require_text CHANGELOG.md "## $cargo_version - Unreleased"
    require_text README.md "The current public release is \`1.3.9\`."
    require_text README.md "The development branch reports package version \`$cargo_version\`"
    require_text README.md '`development-blocked` policy'
    require_text README.md 'base64-ng = { git = "https://github.com/valkyoth/base64-ng"'
    require_text docs/SIMD_ADMISSION.md "Release status: \`1.3.9\`"
    echo "doc versions: ok ($cargo_version development candidate, publishing blocked)"
    exit 0
fi

case "$cargo_version" in
    *-*)
        require_text CHANGELOG.md "## $cargo_version - Unreleased"
        require_text README.md "The development branch is"
        require_text README.md "\`$cargo_version\`"
        require_text docs/SIMD.md "$cargo_version"
        require_text docs/SIMD_ADMISSION.md "$cargo_version"
        ;;
    *)
        require_text README.md "This source tree is the \`$cargo_version\` package-family candidate."
        if [ "$cargo_version" = "1.1.0" ]; then
            require_text docs/SIMD_ADMISSION.md "Release status: \`1.1.x\`"
        else
            require_text docs/SIMD_ADMISSION.md "Release status: \`$cargo_version\`"
        fi
        require_text README.md "base64-ng = \"$cargo_version\""
        reject_text README.md "-alpha"
        reject_text docs/SIMD.md "-alpha"
        reject_text docs/SIMD_ADMISSION.md "-alpha"
        reject_text src/lib.rs "Emerging 2.0 API"
        reject_text src/lib.rs "constant-time-oriented codecs arrive in later 2.0"
        reject_text release-notes/RELEASE_NOTES_2.0.0.md 'base64_ng::v2'
        reject_text docs/RELEASE.md 'git tag -s v1.0.10'
        ;;
esac

echo "doc versions: ok"
