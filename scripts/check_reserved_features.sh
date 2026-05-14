#!/usr/bin/env sh
set -eu

echo "reserved features: tokio remains dependency-free and compile-only"
cargo check --no-default-features --features tokio --lib

echo "reserved features: kani remains dependency-free and compile-only"
cargo check --no-default-features --features kani --lib

echo "reserved features: fuzzing remains dependency-free and compile-only"
cargo check --no-default-features --features fuzzing --lib

echo "reserved features: all reserved features together"
cargo check --no-default-features --features tokio,kani,fuzzing --lib

echo "reserved features: ok"
