#!/usr/bin/env sh
set -eu

workdir="target/migration_smoke"
manifest="$workdir/Cargo.toml"

mkdir -p "$workdir/src"
cp portability/migration_smoke/src/lib.rs "$workdir/src/lib.rs"

cat >"$manifest" <<'MANIFEST'
[package]
name = "base64-ng-migration-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", features = ["alloc", "std", "stream"] }
MANIFEST

cargo test --manifest-path "$manifest" --offline
