#!/usr/bin/env sh
set -eu

expected_tool="cargo-public-api 0.52.0"
public_api_toolchain="nightly-2026-07-13"
snapshot_dir="api-snapshots/v1.3.9"
release_dir="api-snapshots/v2.0.0"
workdir="target/api-snapshot-check"
mode="${1:---check}"

case "$mode" in
    --check | --update)
        ;;
    *)
        echo "api snapshots: usage: $0 [--check|--update]" >&2
        exit 2
        ;;
esac

if ! actual_tool="$(cargo public-api --version 2>/dev/null)"; then
    echo "api snapshots: cargo-public-api 0.52.0 is required" >&2
    exit 1
fi

if [ "$actual_tool" != "$expected_tool" ]; then
    echo "api snapshots: expected $expected_tool, found $actual_tool" >&2
    exit 1
fi

if ! rustup run "$public_api_toolchain" rustc --version >/dev/null 2>&1; then
    echo "api snapshots: Rust $public_api_toolchain is required" >&2
    echo "api snapshots: install it with: rustup toolchain install $public_api_toolchain --profile minimal" >&2
    exit 1
fi

if ! public_api_cargo="$(rustup which --toolchain "$public_api_toolchain" cargo)"; then
    echo "api snapshots: cannot resolve Cargo for $public_api_toolchain" >&2
    exit 1
fi
public_api_path="$(dirname "$public_api_cargo")"
if ! "$public_api_cargo" --version | grep -F -q nightly; then
    echo "api snapshots: $public_api_cargo is not a nightly Cargo binary" >&2
    exit 1
fi

mkdir -p "$workdir"
if [ "$mode" = "--update" ]; then
    mkdir -p "$release_dir"
fi

for package in \
    base64-ng \
    base64-ng-derive \
    base64-ng-imap \
    base64-ng-mime \
    base64-ng-multibase \
    base64-ng-password \
    base64-ng-openpgp \
    base64-ng-pem \
    base64-ng-sanitization \
    base64-ng-serde \
    base64-ng-bytes \
    base64-ng-subtle \
    base64-ng-tokio
do
    generated="$workdir/$package.txt"
    committed="$snapshot_dir/$package.txt"

    echo "api snapshots: generating $package"
    PATH="$public_api_path:$PATH" LC_ALL=C "$public_api_cargo" public-api \
        --color=never \
        --all-features \
        --omit blanket-impls \
        --omit auto-trait-impls \
        -p "$package" >"$generated"

    if [ "$package" != "base64-ng-imap" ] && \
        [ "$package" != "base64-ng-mime" ] && \
        [ "$package" != "base64-ng-multibase" ] && \
        [ "$package" != "base64-ng-password" ] && \
        [ "$package" != "base64-ng-openpgp" ] && \
        [ "$package" != "base64-ng-pem" ] && \
        [ ! -f "$committed" ]; then
        echo "api snapshots: missing $committed" >&2
        exit 1
    fi

    if [ "$package" = "base64-ng" ]; then
        while IFS= read -r baseline_line; do
            if ! grep -F -x -q "$baseline_line" "$generated"; then
                echo "api snapshots: base64-ng removed or changed frozen v1.3.9 API:" >&2
                echo "$baseline_line" >&2
                exit 1
            fi
        done <"$committed"

        development="$release_dir/$package.txt"
        if [ "$mode" = "--update" ]; then
            cp "$generated" "$development"
        elif [ ! -f "$development" ]; then
            echo "api snapshots: missing $development" >&2
            exit 1
        elif ! diff -u "$development" "$generated"; then
            echo "api snapshots: base64-ng drifted from the frozen 2.0.0 API" >&2
            exit 1
        fi
    elif [ "$package" = "base64-ng-derive" ] || \
        [ "$package" = "base64-ng-imap" ] || \
        [ "$package" = "base64-ng-mime" ] || \
        [ "$package" = "base64-ng-multibase" ] || \
        [ "$package" = "base64-ng-password" ] || \
        [ "$package" = "base64-ng-openpgp" ] || \
        [ "$package" = "base64-ng-pem" ] || \
        [ "$package" = "base64-ng-bytes" ] || \
        [ "$package" = "base64-ng-sanitization" ] || \
        [ "$package" = "base64-ng-serde" ] || \
        [ "$package" = "base64-ng-subtle" ] || \
        [ "$package" = "base64-ng-tokio" ]; then
        development="$release_dir/$package.txt"
        if [ "$mode" = "--update" ]; then
            cp "$generated" "$development"
        elif [ ! -f "$development" ]; then
            echo "api snapshots: missing $development" >&2
            exit 1
        elif ! diff -u "$development" "$generated"; then
            echo "api snapshots: $package drifted from the frozen 2.0.0 API" >&2
            exit 1
        fi
    elif ! diff -u "$committed" "$generated"; then
        echo "api snapshots: $package drifted from the v1.3.9 inventory" >&2
        exit 1
    fi
done

echo "api snapshots: frozen 1.3.9 compatibility and 2.0.0 release API ok"
