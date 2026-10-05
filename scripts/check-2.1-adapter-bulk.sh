#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    for features in stream stream,simd stream,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" \
            -p base64-ng -p base64-ng-tokio -p base64-ng-bytes
    done
    cargo +"$compiler" clippy --locked --all-features --all-targets \
        -p base64-ng -p base64-ng-tokio -p base64-ng-bytes -- -D warnings
    cargo +"$compiler" test --locked --no-default-features -p base64-ng-bytes
    cargo +"$compiler" run --locked -p base64-ng-tokio --example stream_transfer
    python3 scripts/test-stream-file-example.py "$compiler"
done
sh scripts/validate-file-line-budget.sh
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 adapter bulk: bounded progress, policies, framing, backpressure and MSRV passed"
