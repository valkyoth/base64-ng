#!/usr/bin/env sh
set -eu

if [ ! -d dudect ]; then
    echo "dudect checks: skipping; dudect/ is not present"
    exit 0
fi

echo "dudect checks: compile timing harness"
cargo check --manifest-path dudect/Cargo.toml --bins

echo "dudect checks: RustSec advisories"
cargo audit --file dudect/Cargo.lock

echo "dudect checks: dependency policy"
cargo deny --manifest-path dudect/Cargo.toml check --config dudect/deny.toml

if [ "${BASE64_NG_RUN_DUDECT:-0}" = "1" ]; then
    samples="${BASE64_NG_DUDECT_SAMPLES:-20000}"
    iterations="${BASE64_NG_DUDECT_ITERS:-64}"
    threshold="${BASE64_NG_DUDECT_THRESHOLD:-10}"
    warmup="${BASE64_NG_DUDECT_WARMUP:-1000}"

    echo "dudect checks: run timing harness"
    cargo run --release --manifest-path dudect/Cargo.toml -- \
        --samples "$samples" \
        --iters "$iterations" \
        --threshold "$threshold" \
        --warmup "$warmup"
else
    echo "dudect checks: timing run skipped; set BASE64_NG_RUN_DUDECT=1 to execute it"
fi
