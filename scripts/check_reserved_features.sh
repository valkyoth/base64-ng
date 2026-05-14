#!/usr/bin/env sh
set -eu

check_reserved_feature() {
    name="$1"
    features="$2"

    echo "reserved features: $name remains dependency-free and compile-only"
    cargo check --no-default-features --features "$features" --lib

    tree_output="$(
        cargo tree --no-default-features --features "$features" --edges normal,build,dev --prefix none --no-dedupe
    )"
    line_count="$(
        printf '%s\n' "$tree_output" | sed '/^[[:space:]]*$/d' | wc -l | tr -d '[:space:]'
    )"

    if [ "$line_count" != "1" ]; then
        echo "reserved features: $name admitted unexpected dependencies" >&2
        printf '%s\n' "$tree_output" >&2
        exit 1
    fi
}

check_reserved_feature "tokio" "tokio"
check_reserved_feature "kani" "kani"
check_reserved_feature "fuzzing" "fuzzing"
check_reserved_feature "all reserved features together" "tokio,kani,fuzzing"

echo "reserved features: ok"
