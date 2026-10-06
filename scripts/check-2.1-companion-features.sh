#!/usr/bin/env sh
set -eu
rustfmt --edition 2024 --check portability/companion_features/*.rs
python3 scripts/test-companion-features.py
python3 scripts/check-companion-features.py
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    # Do not leave another compiler's workspace metadata in the normal cache.
    export CARGO_TARGET_DIR="$PWD/target/companion-workspace/$compiler"
    export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"
    rustup target add thumbv7em-none-eabihf --toolchain "$compiler"
    python3 scripts/check-companion-features.py --toolchain "$compiler" --target thumbv7em-none-eabihf
    # Recovery belongs to the unchanged shared kernels, not duplicated companion code.
    cargo +"$compiler" test --locked --release --no-default-features \
        --features std,checked-backend --lib v2::ordinary_decode::vector::tests
    cargo +"$compiler" test --locked --release --no-default-features \
        --features std,checked-backend --lib encode_backend::checked::tests
    for companion in bytes tokio serde multibase pem openpgp; do
        cargo +"$compiler" test --locked --release -p "base64-ng-$companion" --all-features
        cargo +"$compiler" clippy --locked -p "base64-ng-$companion" --all-targets --all-features -- -D warnings
        cargo +"$compiler" package --locked -p "base64-ng-$companion" --list --allow-dirty > /dev/null
    done
    cargo +"$compiler" run --locked --release -p base64-ng-tokio \
        --features checked-backend --example stream_transfer
done
echo "2.1 companion features: isolated resolution, grammar, recovery and MSRV passed"
