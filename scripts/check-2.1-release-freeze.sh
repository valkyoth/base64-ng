#!/usr/bin/env sh
set -eu

# Package verification only. This gate never publishes or starts campaigns.
python3 scripts/release_crates.py --check
sh scripts/validate-doc-versions.sh
python3 scripts/test-release-freeze.py
python3 scripts/check-2.1-package-inventory.py
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
for compiler in "$active" 1.90.0; do
    (
        export CARGO_TARGET_DIR="$PWD/target/release-freeze-2.1/$compiler"
        export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"
        for features in '' std,simd std,checked-backend; do
            cargo +"$compiler" run --locked --example decode_policy \
                --no-default-features --features "$features"
        done
    )
done
# Core has no unpublished dependencies. Companions require registry order;
# their inventories and workspace tests are checked before publication instead.
cargo package --locked --allow-dirty -p base64-ng
if [ "${BASE64_NG_RUN_COMMIT54_PUBLISH_DRY_RUN:-0}" = "1" ]; then
    cargo publish --locked --allow-dirty --dry-run -p base64-ng
fi
echo "2.1 freeze: inventories, core package build and active/MSRV policy examples passed"
