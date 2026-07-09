#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: scripts/cargo-deny-check.sh <manifest-path> <deny-config>" >&2
    exit 2
fi

manifest="$1"
config="$2"

# cargo-deny has used both of these CLI shapes across versions:
#   cargo deny --manifest-path <path> --config <path> check
#   cargo deny --manifest-path <path> check --config <path>
# Keep release gates portable across developer machines and CI runners.
if cargo deny --help 2>/dev/null | grep -q -- '--config'; then
    cargo deny --manifest-path "$manifest" --config "$config" check
else
    cargo deny --manifest-path "$manifest" check --config "$config"
fi
