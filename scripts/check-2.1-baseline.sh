#!/usr/bin/env sh
set -eu

root="$(pwd)"
baseline=816da2e1e4a66c913057c86d149068f1c88776bf
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for toolchain in "$active" 1.90.0; do
    if ! rustup run "$toolchain" rustc --version >/dev/null 2>&1; then
        echo "2.1 baseline: install required compiler: rustup toolchain install $toolchain --profile minimal" >&2
        exit 1
    fi
done
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$tmp/baseline" "$tmp/consumer/src"
git archive --output="$tmp/baseline.tar" "$baseline"
tar -xf "$tmp/baseline.tar" -C "$tmp/baseline"
cp tests/fixtures/v2_1_consumer.rs "$tmp/consumer/src/lib.rs"

for source in "$tmp/baseline" "$root"; do
    # JSON quoting is also valid for these TOML strings, including spaced paths.
    python3 - "$source" "$tmp/consumer/Cargo.toml" <<'PY'
import json
from pathlib import Path
import sys

Path(sys.argv[2]).write_text('''[package]
name = "base64-ng-compatibility-consumer"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[features]
default = ["std"]
alloc = ["base64-ng/alloc"]
std = ["alloc", "base64-ng/std"]
simd = ["base64-ng/simd"]
checked-backend = ["base64-ng/checked-backend"]
[dependencies]
base64-ng = { path = ''' + json.dumps(sys.argv[1]) + ''', default-features = false }
''')
PY
    for toolchain in "$active" 1.90.0; do
        echo "2.1 baseline: $source with Rust $toolchain"
        cargo +"$toolchain" test --offline --manifest-path "$tmp/consumer/Cargo.toml" \
            --no-default-features
        cargo +"$toolchain" test --offline --manifest-path "$tmp/consumer/Cargo.toml" \
            --all-features
    done
done
echo "2.1 baseline: unchanged downstream fixture passed against release and development"
