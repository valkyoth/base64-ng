#!/usr/bin/env sh
set -eu

toolchain="$(
    sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml | sed -n '1p'
)"

if [ -z "$toolchain" ]; then
    echo "ci rust: rust-toolchain.toml is missing a channel" >&2
    exit 1
fi

add_cargo_path() {
    if [ -n "${GITHUB_PATH:-}" ]; then
        printf '%s\n' "$HOME/.cargo/bin" >> "$GITHUB_PATH"
    fi
    export PATH="$HOME/.cargo/bin:$PATH"
}

install_rustup() {
    case "$(uname -s)" in
        MINGW* | MSYS* | CYGWIN*)
            echo "ci rust: rustup/cargo is broken on Windows; install Rust from the runner image" >&2
            exit 1
            ;;
        *)
            echo "ci rust: installing rustup"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
                | sh -s -- -y --profile minimal --default-toolchain none
            add_cargo_path
            ;;
    esac
}

add_cargo_path

if ! command -v rustup >/dev/null 2>&1; then
    install_rustup
fi

if ! cargo --version >/dev/null 2>&1; then
    echo "ci rust: cargo proxy is not usable before toolchain setup; reinstalling rustup"
    install_rustup
fi

rustup set profile minimal
rustup toolchain install "$toolchain" --component clippy --component rustfmt
rustup default "$toolchain"

if ! cargo --version >/dev/null 2>&1; then
    echo "ci rust: cargo proxy is still not usable after toolchain setup" >&2
    exit 1
fi

rustup show
cargo --version
rustc --version
