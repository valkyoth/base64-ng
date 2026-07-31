#!/usr/bin/env sh
set -eu

workdir="target/feature-contract-smoke"
single_manifest="$workdir/single/Cargo.toml"
unified_manifest="$workdir/unified/Cargo.toml"

mkdir -p "$workdir/single/src" "$workdir/unified/consumer/src"
cp portability/feature_contract_smoke/src/main.rs "$workdir/single/src/main.rs"
cp portability/feature_unification_smoke/src/lib.rs "$workdir/unified/consumer/src/lib.rs"

cat >"$single_manifest" <<'MANIFEST'
[package]
name = "base64-ng-feature-contract-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[features]
default = []
secrets = ["base64-ng/secrets"]
simd = ["base64-ng/simd"]
checked-backend = ["base64-ng/checked-backend"]

[dependencies]
base64-ng = { path = "../../..", default-features = false }
MANIFEST

cat >"$unified_manifest" <<'MANIFEST'
[workspace]
members = [
    "ordinary-provider",
    "accelerated-provider",
    "secret-provider",
    "checked-provider",
    "consumer",
]
resolver = "2"
MANIFEST

for provider in ordinary accelerated secret checked; do
    mkdir -p "$workdir/unified/$provider-provider/src"
    printf '%s\n' 'pub use base64_ng::*;' >"$workdir/unified/$provider-provider/src/lib.rs"
done

cat >"$workdir/unified/ordinary-provider/Cargo.toml" <<'MANIFEST'
[package]
name = "ordinary-provider"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
base64-ng = { path = "../../../..", default-features = false }
MANIFEST

cat >"$workdir/unified/accelerated-provider/Cargo.toml" <<'MANIFEST'
[package]
name = "accelerated-provider"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
base64-ng = { path = "../../../..", default-features = false, features = ["simd"] }
MANIFEST

cat >"$workdir/unified/secret-provider/Cargo.toml" <<'MANIFEST'
[package]
name = "secret-provider"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
base64-ng = { path = "../../../..", default-features = false, features = ["secrets"] }
MANIFEST

cat >"$workdir/unified/checked-provider/Cargo.toml" <<'MANIFEST'
[package]
name = "checked-provider"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
base64-ng = { path = "../../../..", default-features = false, features = ["checked-backend"] }
MANIFEST

cat >"$workdir/unified/consumer/Cargo.toml" <<'MANIFEST'
[package]
name = "base64-ng-feature-unification-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
ordinary-provider = { path = "../ordinary-provider" }
accelerated-provider = { path = "../accelerated-provider" }
secret-provider = { path = "../secret-provider" }
checked-provider = { path = "../checked-provider" }
MANIFEST

baseline="$(
    cargo run --quiet --offline --manifest-path "$single_manifest"
)"

for features in \
    secrets \
    simd \
    secrets,simd \
    checked-backend \
    secrets,checked-backend
do
    echo "2.0 feature contract: checking $features"
    observed="$(
        cargo run --quiet --offline --manifest-path "$single_manifest" --features "$features"
    )"
    if [ "$observed" != "$baseline" ]; then
        echo "2.0 feature contract: ordinary ABI changed under $features" >&2
        echo "baseline: $baseline" >&2
        echo "observed: $observed" >&2
        exit 1
    fi
done

echo "2.0 feature contract: checking downstream feature unification"
cargo test --offline --manifest-path "$unified_manifest" -p base64-ng-feature-unification-smoke

echo "2.0 feature contract: checking all-features workspace"
cargo test --all-features

echo "2.0 feature contract: ok ($baseline)"
