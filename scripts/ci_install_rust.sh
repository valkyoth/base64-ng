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

add_ci_cargo_wrapper() {
    if [ -z "${GITHUB_PATH:-}" ]; then
        return
    fi

    wrapper_dir="${RUNNER_TEMP:-/tmp}/base64-ng-rust-bin"
    mkdir -p "$wrapper_dir"
    {
        printf '%s\n' '#!/usr/bin/env sh'
        printf '%s\n' 'case "${1:-}" in'
        printf '%s\n' '    +*)'
        printf '%s\n' '        toolchain="${1#+}"'
        printf '%s\n' '        shift'
        printf '%s\n' '        exec rustup run "$toolchain" cargo "$@"'
        printf '%s\n' '        ;;'
        printf '%s\n' 'esac'
        printf 'exec rustup run %s cargo "$@"\n' "$toolchain"
    } > "$wrapper_dir/cargo"
    chmod +x "$wrapper_dir/cargo"

    printf '%s\n' "$wrapper_dir" >> "$GITHUB_PATH"
    export PATH="$wrapper_dir:$PATH"
}

add_cargo_path

if ! command -v rustup >/dev/null 2>&1; then
    echo "ci rust: rustup is required from the runner image; refusing curl-pipe-shell bootstrap" >&2
    exit 1
fi

if ! cargo --version >/dev/null 2>&1; then
    echo "ci rust: cargo proxy is not usable before toolchain setup" >&2
    exit 1
fi

rustup set profile minimal
rustup toolchain install "$toolchain" --component clippy --component rustfmt
rustup default "$toolchain"
add_ci_cargo_wrapper

if ! cargo --version >/dev/null 2>&1; then
    echo "ci rust: cargo proxy is still not usable after toolchain setup" >&2
    exit 1
fi

active_rust_version="$(rustc --version | awk '{print $2}')"
if [ "$active_rust_version" != "$toolchain" ]; then
    echo "ci rust: expected rustc $toolchain, got $active_rust_version" >&2
    exit 1
fi

rustup show
cargo --version
rustc --version
