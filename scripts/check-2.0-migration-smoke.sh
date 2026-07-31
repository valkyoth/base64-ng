#!/usr/bin/env sh
set -eu

workdir="target/2_0_migration_smoke"
manifest="$workdir/Cargo.toml"

mkdir -p "$workdir/src"
cp portability/2_0_migration_smoke/src/lib.rs "$workdir/src/lib.rs"

cat >"$manifest" <<'MANIFEST'
[package]
name = "base64-ng-2-0-migration-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", features = ["alloc", "std", "stream", "secrets"] }
MANIFEST

cargo test --offline --manifest-path "$manifest"

echo "2.0 migration smoke: ok"
