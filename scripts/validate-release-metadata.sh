#!/usr/bin/env sh
set -eu

package_name="$(
    sed -n 's/^name = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
cargo_version="$(
    sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
cargo_rust_version="$(
    sed -n 's/^rust-version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
toolchain_version="$(
    sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml | sed -n '1p'
)"

if [ "$package_name" != "base64-ng" ]; then
    echo "release metadata: package name must be base64-ng" >&2
    exit 1
fi

if [ -z "$cargo_version" ]; then
    echo "release metadata: Cargo.toml package version is missing" >&2
    exit 1
fi

if [ -z "$cargo_rust_version" ]; then
    echo "release metadata: Cargo.toml rust-version is missing" >&2
    exit 1
fi

if [ "$toolchain_version" != "$cargo_rust_version.0" ]; then
    echo "release metadata: rust-toolchain.toml channel $toolchain_version does not match Cargo.toml rust-version $cargo_rust_version" >&2
    exit 1
fi

if ! grep -q '^license = "MIT OR Apache-2.0"$' Cargo.toml; then
    echo "release metadata: Cargo.toml must declare license = \"MIT OR Apache-2.0\"" >&2
    exit 1
fi

test -s LICENSE-MIT
test -s LICENSE-APACHE
test -s README.md
test -s SECURITY.md

if ! grep -q "^## $cargo_version " CHANGELOG.md; then
    echo "release metadata: CHANGELOG.md is missing a section for Cargo version $cargo_version" >&2
    exit 1
fi

echo "release metadata: ok"
