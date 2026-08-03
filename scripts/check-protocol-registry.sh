#!/usr/bin/env sh
set -eu

manifest="protocol-registry/runner/Cargo.toml"
msrv_toolchain="${BASE64_NG_MSRV_TOOLCHAIN:-1.90.0}"

echo "protocol registry: locked sources and complete public-name inventory"
scripts/validate-password-spec.py
scripts/verify-rfcs.sh
scripts/validate-multibase-spec.py
scripts/validate-protocol-registry.py
scripts/check-protocol-registry-mutations.py

echo "protocol registry: independent models, production parsers, and references"
cargo run --quiet --locked --manifest-path "$manifest"
cargo clippy --locked --manifest-path "$manifest" --all-targets -- -D warnings

echo "protocol registry: dependency provenance"
cargo audit --file protocol-registry/runner/Cargo.lock
scripts/cargo-deny-check.sh "$manifest" protocol-registry/runner/deny.toml

if rustup run "$msrv_toolchain" rustc --version >/dev/null 2>&1; then
    echo "protocol registry: optional local MSRV evidence ($msrv_toolchain)"
    cargo +"$msrv_toolchain" check --locked --manifest-path "$manifest"
else
    echo "protocol registry: skipping local MSRV check; Rust $msrv_toolchain is not installed"
    echo "protocol registry: the dedicated CI MSRV matrix remains authoritative"
fi

echo "protocol registry: package exclusion"
package_list="$(mktemp)"
trap 'rm -f "$package_list"' EXIT HUP INT TERM
cargo package --locked --allow-dirty --list -p base64-ng >"$package_list"
if grep -E '(^|/)(protocol-registry|spec/password)/' "$package_list"; then
    echo "protocol registry: retained evidence leaked into base64-ng package" >&2
    exit 1
fi

echo "protocol registry: claims, corpus, models, provenance, and package scope ok"
