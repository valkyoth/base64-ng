#!/usr/bin/env sh
set -eu

workdir="target/no_alloc_smoke"
manifest="$workdir/Cargo.toml"
targets="${*:-x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf}"
installed="$(rustup target list --installed)"

mkdir -p "$workdir/src"
cp portability/no_alloc_smoke/src/lib.rs "$workdir/src/lib.rs"

cat > "$manifest" <<'MANIFEST'
[package]
name = "base64-ng-no-alloc-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["allow-wasm32-best-effort-wipe"] }
MANIFEST

cargo test --manifest-path "$manifest" --offline
cargo check --manifest-path "$manifest" --offline

for target in $targets; do
    if printf '%s\n' "$installed" | grep -qx "$target"; then
        echo "no-alloc smoke: checking $target"
        cargo check --manifest-path "$manifest" --offline --target "$target"
    else
        echo "no-alloc smoke: skipping $target; Rust target is not installed"
    fi
done
