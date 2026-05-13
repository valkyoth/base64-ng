#!/usr/bin/env sh
set -eu

if [ ! -d perf ]; then
    echo "perf checks: skipping; perf/ is not present"
    exit 0
fi

echo "perf checks: compile benchmark harness"
cargo check --manifest-path perf/Cargo.toml --bins

echo "perf checks: RustSec advisories"
cargo audit --file perf/Cargo.lock

echo "perf checks: dependency policy"
cargo deny --manifest-path perf/Cargo.toml check --config perf/deny.toml
