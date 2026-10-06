#!/usr/bin/env sh
set -eu
case "${1-}" in
    ''|--extended) ;;
    *) echo "usage: sh scripts/check-2.1-security.sh [--extended]" >&2; exit 2 ;;
esac
test "$#" -le 1
ulimit -c 0 2>/dev/null || true
root="$PWD"
active="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
python3 scripts/validate-kani-proof-inventory.py
python3 scripts/test-security-gate.py
python3 scripts/test-x86-validation-asm.py
python3 scripts/test-x86-validation-ir.py
python3 scripts/test-neon-validation-codegen.py
python3 scripts/test-rvv-validation-codegen.py
python3 scripts/test-incremental-decoder-policy.py
python3 scripts/test-companion-features.py
for compiler in "$active" 1.90.0; do
    python3 scripts/test-decode-preflight-borrows.py "$compiler"
    export CARGO_TARGET_DIR="$root/target/security-2.1/$compiler"
    export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"
    for features in '' std,simd std,checked-backend; do
        cargo +"$compiler" test --locked --release --no-default-features --features "$features" \
            --test decode_ref --test decode_validation --test in_place_bulk --test incremental_bulk
    done
    cargo +"$compiler" test --locked --release --all-features --lib v2::ordinary_decode
    cargo +"$compiler" test --locked --release --all-features --lib validation_guard_tests
    cargo +"$compiler" test --locked --release --all-features --lib v2::secret_
    cargo +"$compiler" test --locked --release -p base64-ng-tokio --all-features --test bulk_adapters
    RUSTFLAGS="--cfg fuzzing" cargo +"$compiler" test --locked --manifest-path fuzz/Cargo.toml --lib
    RUSTFLAGS="--cfg fuzzing" cargo +"$compiler" clippy --locked --manifest-path fuzz/Cargo.toml \
        --lib --bins -- -D warnings
    cargo +"$compiler" clippy --locked --all-features --all-targets -- -D warnings
done
sh scripts/validate-unsafe-boundary.sh
sh scripts/validate-panic-policy.sh
sh scripts/validate-file-line-budget.sh
echo "2.1 security: deterministic regressions and mutation checks passed"
if [ "${1-}" != --extended ]; then
    echo "2.1 security: Miri, Kani and sanitizer/fuzz smoke not requested (use --extended)"
    exit 0
fi

python3 scripts/check-companion-features.py

# Explicit local verification only; do not silently count unavailable tools as passes.
(
# Copied Miri caches can retain absolute runner paths from another checkout.
mkdir -p "$root/target/security-2.1"
miri_target="$(mktemp -d "$root/target/security-2.1/miri.XXXXXX")"
trap 'rm -rf "$miri_target"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export CARGO_TARGET_DIR="$miri_target"
unset CARGO_BUILD_BUILD_DIR
export CARGO_INCREMENTAL=0
cargo +nightly miri test --locked --all-features --test decode_ref \
    borrowed_view_security_retry_keeps_proof_and_entire_destination -- --exact
cargo +nightly miri test --locked --all-features --lib v2::ordinary_decode::retained::tests
cargo +nightly miri test --locked --all-features --lib \
    v2::ordinary_decode::in_place::tests::in_place_bulk_miri_preserved_source_and_scalar_repair -- --exact
)
(
mkdir -p "$root/target/security-2.1"
kani_target="$(mktemp -d "$root/target/security-2.1/kani.XXXXXX")"
trap 'rm -rf "$kani_target"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export CARGO_TARGET_DIR="$kani_target"
unset CARGO_BUILD_BUILD_DIR
export CARGO_INCREMENTAL=0
for proof in ordinary_table_validation_refines_scalar_for_two_quanta bulk_in_place_chunk_geometry_preserves_unread_suffix; do
    (
        ulimit -v 8388608
        cargo +1.90.0 kani --no-default-features -Z unstable-options --harness-timeout 5m --harness "$proof"
    )
done
)
test "$(uname -sm)" = 'Linux x86_64'
export CARGO_TARGET_DIR="$root/target/security-2.1/asan"
export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test --locked -Zbuild-std \
    --target x86_64-unknown-linux-gnu --all-features \
    --test decode_ref --test decode_validation --test in_place_bulk --test incremental_bulk

export CARGO_TARGET_DIR="$root/target/security-2.1/fuzz"
export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"
corpus="$(mktemp -d "$CARGO_TARGET_DIR-corpus.XXXXXX")"
trap 'rm -rf "$corpus"' EXIT INT TERM
python3 - "$corpus" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
(root / "in_place").mkdir()
(root / "in_place/bulk-valid").write_bytes(b'A' * 4096)
(root / "in_place/bulk-high-bit").write_bytes(b'A' * 4095 + b'\x80')
(root / "in_place/tail-bits").write_bytes(b'A' * 4092 + b'Zh==')
(root / "v2_incremental").mkdir()
for path in Path('fuzz/corpus/v2_incremental').glob('bulk-*'):
    (root / "v2_incremental" / path.name).write_bytes(path.read_bytes())
assert any((root / "v2_incremental").iterdir()), 'missing committed bulk seeds'
PY
for target in in_place v2_incremental; do
    cargo +nightly fuzz run "$target" "$corpus/$target" -- \
        -runs=1000 -max_len=8192 -timeout=20 -rss_limit_mb=2048 -print_final_stats=1
done
echo "2.1 security: Miri, bounded Kani, native ASan and 1000-run seeded fuzz smokes passed; not release campaigns"
