#!/usr/bin/env sh
set -eu

python3 scripts/validate-semantic-corpus.py
cargo run --quiet --locked --manifest-path semantic-corpus/runner/Cargo.toml
cargo audit --file semantic-corpus/runner/Cargo.lock
scripts/cargo-deny-check.sh \
    semantic-corpus/runner/Cargo.toml \
    semantic-corpus/runner/deny.toml

echo "semantic corpus: cross-crate runner ok"
