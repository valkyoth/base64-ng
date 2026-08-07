#!/usr/bin/env sh
set -eu

matrix="docs/2.0_DISPATCH_AND_PERFORMANCE_MATRIX.md"
manifest="performance-baselines/dispatch-commit-34-amd-9950x3d-linux/MANIFEST.txt"
encode_policy="src/encode_backend/policy.rs"
decode_policy="src/decode_backend/policy.rs"

test -s "$matrix"
test -s "$manifest"

for required in \
    '| x86/x86_64 | encode | SSSE3/SSE4.1 | 12 |' \
    '| x86/x86_64 | encode | AVX2 | 24 |' \
    '| x86/x86_64 | encode | AVX-512 VBMI | 192 |' \
    '| x86/x86_64 | strict decode | SSSE3/SSE4.1 | 16 |' \
    '| x86/x86_64 | strict decode | AVX2 | 32 |' \
    '| x86/x86_64 | strict decode | AVX-512 VBMI | - |' \
    '| little-endian AArch64 | encode | NEON | 192 |' \
    '| little-endian AArch64 | strict decode | NEON | 256 |' \
    '| Linux SpacemiT X60 | encode | RVV 1.0 | 384 |' \
    '| Linux SpacemiT X60 | strict decode | RVV 1.0 | 1024 |' \
    '| AArch64 Linux/Android | encode/decode | SVE | - |'
do
    if ! grep -F -q "$required" "$matrix"; then
        echo "2.0 dispatch matrix: missing row: $required" >&2
        exit 1
    fi
done

for required in \
    'pub(super) const MIN_SIMD_INPUT: usize = 12;' \
    'pub(super) const X86_AVX2_MIN_INPUT: usize = 24;' \
    'pub(super) const X86_AVX512_MIN_INPUT: usize = 192;' \
    'pub(super) const NEON_MIN_INPUT: usize = 192;' \
    'pub(super) const RVV_MIN_INPUT: usize = 384;'
do
    grep -F -q "$required" "$encode_policy"
done

for required in \
    'pub(super) const MIN_SIMD_INPUT: usize = 16;' \
    'pub(super) const X86_AVX2_MIN_INPUT: usize = 32;' \
    'pub(super) const NEON_MIN_INPUT: usize = 256;' \
    'pub(super) const RVV_MIN_INPUT: usize = 1024;'
do
    grep -F -q "$required" "$decode_policy"
done

if grep -F -q 'X86_AVX512_MIN_INPUT' "$decode_policy"; then
    echo "2.0 dispatch matrix: strict decode must not invent an AVX-512 automatic threshold" >&2
    exit 1
fi

grep -F -q 'samples_per_cell=15' "$manifest"
grep -F -q 'median_minimum_ratio=1.02' "$manifest"
grep -F -q 'one_sided_sign_test_maximum_p=0.05' "$manifest"

(
    cd "$(dirname "$manifest")"
    sha256sum -c CHECKSUMS.sha256 --strict
)

scripts/test-x86-encode-performance.py
scripts/test-x86-decode-performance.py
scripts/test-neon-performance.py
scripts/validate-x86-encode-performance.py \
    performance-baselines/dispatch-commit-34-amd-9950x3d-linux/x86-encode.csv
scripts/validate-x86-decode-performance.py \
    performance-baselines/dispatch-commit-34-amd-9950x3d-linux/x86-decode.csv

cargo test --all-features --lib 'every_x86_encode_threshold_and_downgrade_edge_is_forced'
cargo test --all-features --lib 'every_x86_decode_threshold_and_downgrade_edge_is_forced'
cargo check --no-default-features --features simd --lib

echo "2.0 dispatch matrix: thresholds, downgrades, statistics, and static fallback ok"
