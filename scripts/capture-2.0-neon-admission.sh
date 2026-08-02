#!/usr/bin/env sh
set -eu

output_dir="${1:-target/release-evidence/commit-34-neon}"
samples="${BASE64_NG_NEON_SAMPLES:-15}"
target_bytes="${BASE64_NG_NEON_TARGET_BYTES:-16777216}"

host="$(rustc -vV | sed -n 's/^host: //p')"
case "$host" in
    aarch64-*)
        ;;
    *)
        echo "NEON admission capture: requires an AArch64 host, got $host" >&2
        exit 1
        ;;
esac

if ! rustc --print cfg | grep -F -q 'target_endian="little"'; then
    echo "NEON admission capture: requires little-endian AArch64" >&2
    exit 1
fi
if [ "$samples" -lt 15 ]; then
    echo "NEON admission capture: at least 15 samples are required" >&2
    exit 1
fi
if [ -e "$output_dir" ]; then
    echo "NEON admission capture: output already exists: $output_dir" >&2
    exit 1
fi
if [ -n "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    echo "NEON admission capture: source tree must be clean" >&2
    exit 1
fi

source_commit="$(git rev-parse HEAD^{commit})"
temporary="${output_dir}.tmp.$$"
trap 'rm -rf "$temporary"' 0 1 2 15
mkdir -p "$temporary"

echo "NEON admission capture: correctness before measurement"
cargo test --all-features --lib 'simd::neon_direct_tests'
cargo test --all-features --lib 'simd::neon_decode_tests'
cargo test --all-features --lib 'simd::tests::neon_encode_block'

echo "NEON admission capture: 15-sample exact-backend matrix"
BASE64_NG_PERF_SAMPLES="$samples" \
BASE64_NG_PERF_TARGET_BYTES="$target_bytes" \
RUSTFLAGS='--cfg base64_ng_perf_evidence' \
    cargo run --quiet --release --manifest-path perf/Cargo.toml -- neon \
    >"$temporary/neon.csv"
scripts/validate-neon-performance.py "$temporary/neon.csv"

echo "NEON admission capture: correctness after measurement"
cargo test --all-features --lib 'simd::neon_direct_tests'
cargo test --all-features --lib 'simd::neon_decode_tests'

if [ "$source_commit" != "$(git rev-parse HEAD^{commit})" ] \
    || [ -n "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    echo "NEON admission capture: source changed during measurement" >&2
    exit 1
fi

rustc -Vv >"$temporary/rustc.txt"
uname -a >"$temporary/uname.txt"
if command -v lscpu >/dev/null 2>&1; then
    lscpu >"$temporary/cpu.txt"
elif command -v sysctl >/dev/null 2>&1; then
    sysctl -a >"$temporary/cpu.txt" 2>/dev/null || true
fi

cat >"$temporary/MANIFEST.txt" <<EOF
schema=base64-ng-neon-performance-v1
source_commit=$source_commit
source_status=clean
host=$host
samples_per_cell=$samples
target_bytes_per_sample=$target_bytes
median_minimum_ratio=1.02
one_sided_sign_test_maximum_p=0.05
EOF

(
    cd "$temporary"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum MANIFEST.txt cpu.txt neon.csv rustc.txt uname.txt \
            2>/dev/null >CHECKSUMS.sha256
    else
        shasum -a 256 MANIFEST.txt cpu.txt neon.csv rustc.txt uname.txt \
            2>/dev/null >CHECKSUMS.sha256
    fi
)

mkdir -p "$(dirname "$output_dir")"
mv "$temporary" "$output_dir"
trap - 0 1 2 15
echo "NEON admission capture: wrote $output_dir"
