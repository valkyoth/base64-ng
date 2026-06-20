#!/usr/bin/env sh
set -eu

check_reserved_feature() {
    name="$1"
    features="$2"

    echo "reserved features: $name remains dependency-free and placeholder-only"
    cargo check --no-default-features --features "$features" --lib

    tree_output="$(
        cargo tree -p base64-ng --no-default-features --features "$features" --edges normal,build,dev --prefix none --no-dedupe
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

for inert_feature in tokio kani fuzzing; do
    if ! grep -q "^$inert_feature = \\[\\]$" Cargo.toml; then
        echo "reserved features: $inert_feature must remain an inert Cargo feature" >&2
        exit 1
    fi
done

for deferred_feature in serde bytes zeroize subtle criterion; do
    if grep -q "^$deferred_feature =" Cargo.toml; then
        echo "reserved features: $deferred_feature must not be exposed before dependency admission" >&2
        exit 1
    fi
done

check_reserved_feature "tokio" "tokio"
check_reserved_feature "kani" "kani"
check_reserved_feature "fuzzing" "fuzzing"
check_reserved_feature "all reserved features together" "tokio,kani,fuzzing"

echo "reserved features: ok"
