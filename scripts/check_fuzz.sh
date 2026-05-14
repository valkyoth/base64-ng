#!/usr/bin/env sh
set -eu

if [ ! -d fuzz ]; then
    echo "fuzz checks: skipping; fuzz/ is not present"
    exit 0
fi

echo "fuzz checks: compile harnesses"
cargo check --manifest-path fuzz/Cargo.toml --bins

echo "fuzz checks: corpus policy"
scripts/check_fuzz_corpus.sh

echo "fuzz checks: RustSec advisories"
cargo audit --file fuzz/Cargo.lock

echo "fuzz checks: dependency policy"
cargo deny --manifest-path fuzz/Cargo.toml check --config fuzz/deny.toml
