#!/usr/bin/env sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
    echo "macOS checks: this script must be run on macOS" >&2
    exit 1
fi

toolchain="$(rustup show active-toolchain | sed 's/ .*//')"
rustc_path="$(rustup which --toolchain "$toolchain" rustc)"
host="$(rustup run "$toolchain" rustc -vV | sed -n 's/^host: //p')"
machine="$(uname -m)"
script_revision="2026-05-29-host-target-v3"

cargo_check() {
    RUSTC="$rustc_path" rustup run "$toolchain" cargo "$@"
}

echo "macOS checks: script=$script_revision"
echo "macOS checks: host=$host machine=$machine toolchain=$toolchain"
echo "macOS checks: rustc=$(rustup run "$toolchain" rustc --version)"
echo "macOS checks: cargo=$(rustup run "$toolchain" cargo --version)"
echo "macOS checks: rustc path=$rustc_path"

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

if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
    echo "macOS checks: CARGO_BUILD_TARGET=$CARGO_BUILD_TARGET is set; host checks force --target $host"
fi

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    if ! rustup target list --installed --toolchain "$toolchain" | grep -qx "$target"; then
        echo "macOS checks: installing missing Rust target $target for $toolchain"
        rustup target add --toolchain "$toolchain" "$target"
    fi

    if ! target_libdir="$(rustup run "$toolchain" rustc --target "$target" --print target-libdir 2>/dev/null)"; then
        echo "macOS checks: Rust target $target is not usable for $toolchain" >&2
        echo "macOS checks: installed targets for $toolchain:" >&2
        rustup target list --installed --toolchain "$toolchain" >&2
        exit 1
    fi

    if ! ls "$target_libdir"/libstd-*.rlib >/dev/null 2>&1; then
        echo "macOS checks: target $target is installed but std is missing for $toolchain" >&2
        echo "macOS checks: target libdir: $target_libdir" >&2
        echo "macOS checks: try: rustup target remove --toolchain $toolchain $target" >&2
        echo "macOS checks: then: rustup target add --toolchain $toolchain $target" >&2
        exit 1
    fi

    echo "macOS checks: target $target std ok at $target_libdir"
done

echo "macOS checks: host test default features"
cargo_check test --target "$host" --all-targets

echo "macOS checks: host test all features"
cargo_check test --target "$host" --all-targets --all-features

echo "macOS checks: host check no_std library"
cargo_check check --target "$host" --no-default-features --lib

echo "macOS checks: host test no default features"
cargo_check test --target "$host" --no-default-features --all-targets

echo "macOS checks: host clippy all features"
cargo_check clippy --target "$host" --all-targets --all-features -- -D warnings

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    echo "macOS checks: target compile all features for $target"
    cargo_check check --target "$target" --all-features --lib

    echo "macOS checks: target compile no default features for $target"
    cargo_check check --target "$target" --no-default-features --lib
done

echo "macOS checks: ok"
