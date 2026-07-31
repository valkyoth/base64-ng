#!/usr/bin/env sh
set -eu

check_reserved_feature() {
    name="$1"
    features="$2"

    echo "reserved features: checking dependency boundary for $name"
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

if ! grep -q '^secrets = \[\]$' Cargo.toml; then
    echo "reserved features: secrets must remain dependency-free" >&2
    exit 1
fi
if ! grep -F -q 'pub mod secret;' src/v2/mod.rs; then
    echo "reserved features: secrets storage capability is not public" >&2
    exit 1
fi

for deferred_feature in serde bytes zeroize subtle criterion; do
    if grep -q "^$deferred_feature =" Cargo.toml; then
        echo "reserved features: $deferred_feature must not be exposed before dependency admission" >&2
        exit 1
    fi
done

check_reserved_feature "tokio" "tokio"
check_reserved_feature "kani" "kani"
check_reserved_feature "fuzzing" "fuzzing"
check_reserved_feature "secrets storage capability" "secrets"

if ! grep -q '^checked-backend = \["simd"\]$' Cargo.toml; then
    echo "reserved features: checked-backend must imply simd exactly" >&2
    exit 1
fi

check_reserved_feature "checked-backend capability reservation" "checked-backend"
check_reserved_feature \
    "all reserved features together" \
    "tokio,kani,fuzzing,secrets,checked-backend"

echo "reserved features: ok"
