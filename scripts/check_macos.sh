#!/usr/bin/env sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
    echo "macOS checks: this script must be run on macOS" >&2
    exit 1
fi

toolchain="$(rustup show active-toolchain | sed 's/ .*//')"
host="$(rustc +"$toolchain" -vV | sed -n 's/^host: //p')"
machine="$(uname -m)"

echo "macOS checks: host=$host machine=$machine toolchain=$toolchain"
echo "macOS checks: rustc=$(rustc +"$toolchain" --version)"
echo "macOS checks: cargo=$(cargo +"$toolchain" --version)"

case "$host" in
    aarch64-apple-darwin|x86_64-apple-darwin)
        ;;
    *)
        echo "macOS checks: unexpected Rust host triple: $host" >&2
        exit 1
        ;;
esac

if [ "$machine" = "arm64" ] && [ "$host" != "aarch64-apple-darwin" ]; then
    echo "macOS checks: expected aarch64-apple-darwin Rust host on arm64 Mac" >&2
    exit 1
fi

if [ "$machine" = "x86_64" ] && [ "$host" != "x86_64-apple-darwin" ]; then
    echo "macOS checks: expected x86_64-apple-darwin Rust host on Intel Mac" >&2
    exit 1
fi

echo "macOS checks: host test default features"
cargo +"$toolchain" test --all-targets

echo "macOS checks: host test all features"
cargo +"$toolchain" test --all-targets --all-features

echo "macOS checks: host check no_std library"
cargo +"$toolchain" check --no-default-features --lib

echo "macOS checks: host test no default features"
cargo +"$toolchain" test --no-default-features --all-targets

echo "macOS checks: host clippy all features"
cargo +"$toolchain" clippy --all-targets --all-features -- -D warnings

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    if ! rustup target list --installed --toolchain "$toolchain" | grep -qx "$target"; then
        echo "macOS checks: installing missing Rust target $target for $toolchain"
        rustup target add --toolchain "$toolchain" "$target"
    fi

    if ! rustc +"$toolchain" --target "$target" --print target-libdir >/dev/null 2>&1; then
        echo "macOS checks: Rust target $target is not usable for $toolchain" >&2
        echo "macOS checks: installed targets for $toolchain:" >&2
        rustup target list --installed --toolchain "$toolchain" >&2
        exit 1
    fi

    echo "macOS checks: target compile all features for $target"
    cargo +"$toolchain" check --target "$target" --all-features --lib

    echo "macOS checks: target compile no default features for $target"
    cargo +"$toolchain" check --target "$target" --no-default-features --lib
done

echo "macOS checks: ok"
