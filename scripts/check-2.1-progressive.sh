#!/usr/bin/env sh
set -eu
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
python3 scripts/test-2.0-skeleton.py
python3 scripts/test-progressive-measurement.py
for compiler in "$active" 1.90.0; do
    export CARGO_TARGET_DIR="$PWD/target/progressive-workspace/$compiler"
    export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"
    for features in std,simd std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --lib
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" --test decode_validation --test incremental_bulk
        cargo +"$compiler" clippy --locked --no-default-features --features "$features" --lib --tests -- -D warnings
    done
    cargo +"$compiler" check --locked --no-default-features --lib
    cargo +"$compiler" rustc --locked --release --all-features --lib -- --emit=llvm-ir
    python3 - "$CARGO_TARGET_DIR" <<'PY'
from pathlib import Path
import sys
files = list(Path(sys.argv[1]).rglob('*.ll'))
assert files, 'no production LLVM IR generated'
for path in files:
    assert 'progressive_candidate' not in path.read_text(), f'prototype leaked into {path}'
PY
done
sh scripts/validate-file-line-budget.sh
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
echo "2.1 progressive: no-go removal, retained evidence, transactional/incremental alternatives and MSRV passed"
