#!/usr/bin/env sh
set -eu

deny_advisories() {
    manifest="$1"
    config="$2"

    if cargo deny --help 2>/dev/null | grep -q -- '--config'; then
        cargo deny --manifest-path "$manifest" --config "$config" check advisories
    else
        cargo deny --manifest-path "$manifest" check advisories --config "$config"
    fi
}

echo "scheduled advisories: workspace RustSec audit"
cargo audit --file Cargo.lock

echo "scheduled advisories: workspace cargo-deny advisory policy"
deny_advisories Cargo.toml deny.toml

for package in dudect fuzz perf; do
    echo "scheduled advisories: $package RustSec audit"
    cargo audit --file "$package/Cargo.lock"

    echo "scheduled advisories: $package cargo-deny advisory policy"
    deny_advisories "$package/Cargo.toml" "$package/deny.toml"
done

echo "scheduled advisories: ok"
