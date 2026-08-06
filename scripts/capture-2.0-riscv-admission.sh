#!/usr/bin/env sh
set -eu

output_dir="${1:-target/release-evidence/riscv-native-admission}"
samples="${BASE64_NG_RVV_SAMPLES:-15}"
target_bytes="${BASE64_NG_RVV_TARGET_BYTES:-4194304}"
expected_mvendorid="0x710"
expected_marchid="0x8000000058000001"
expected_mimpid="0x1000000049772200"

cpuinfo_value() {
    sed -n "s/^$1[[:space:]]*:[[:space:]]*//p" /proc/cpuinfo | sed -n '1p'
}

host="$(rustc -vV | sed -n 's/^host: //p')"
case "$host" in
    riscv64*-linux-gnu) ;;
    *)
        echo "RVV admission capture: requires native riscv64 Linux, got $host" >&2
        exit 1
        ;;
esac
if grep -E -i -q 'qemu|emulator' /proc/cpuinfo 2>/dev/null; then
    echo "RVV admission capture: emulated CPUs are not accepted" >&2
    exit 1
fi
if command -v systemd-detect-virt >/dev/null 2>&1 && systemd-detect-virt --quiet; then
    echo "RVV admission capture: virtualized execution is not accepted" >&2
    exit 1
fi
mvendorid="$(cpuinfo_value mvendorid)"
marchid="$(cpuinfo_value marchid)"
mimpid="$(cpuinfo_value mimpid)"
if [ "$mvendorid" != "$expected_mvendorid" ] \
    || [ "$marchid" != "$expected_marchid" ] \
    || [ "$mimpid" != "$expected_mimpid" ]; then
    echo "RVV admission capture: host is not the reviewed SpacemiT X60 profile" >&2
    exit 1
fi
if [ "$samples" -lt 15 ]; then
    echo "RVV admission capture: at least 15 samples are required" >&2
    exit 1
fi
if [ -e "$output_dir" ]; then
    echo "RVV admission capture: output already exists: $output_dir" >&2
    exit 1
fi
if [ -n "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    echo "RVV admission capture: source tree must be clean" >&2
    exit 1
fi

source_commit="$(git rev-parse HEAD^{commit})"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-rvv-admission.XXXXXX")"
trap 'rm -rf "$temporary"' EXIT INT TERM

echo "RVV admission capture: native correctness, signal, thread, and assembly evidence"
scripts/check_riscv_hardware.sh >"$temporary/correctness.txt" 2>&1
cat "$temporary/correctness.txt"

echo "RVV admission capture: 15-sample exact-backend matrix"
BASE64_NG_PERF_SAMPLES="$samples" \
BASE64_NG_PERF_TARGET_BYTES="$target_bytes" \
RUSTFLAGS='--cfg base64_ng_perf_evidence --cfg base64_ng_rvv_candidate' \
    cargo run --quiet --release --manifest-path perf/Cargo.toml -- rvv \
    >"$temporary/rvv.csv"
scripts/validate-rvv-performance.py "$temporary/rvv.csv"

if [ "$source_commit" != "$(git rev-parse HEAD^{commit})" ] \
    || [ -n "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    echo "RVV admission capture: source changed during measurement" >&2
    exit 1
fi

cp target/release-evidence/rvv-asm/disassembly.txt "$temporary/asm-disassembly.txt"
cp target/release-evidence/rvv-asm/attributes.txt "$temporary/asm-attributes.txt"
cp target/release-evidence/rvv-asm/MANIFEST.txt "$temporary/asm-manifest.txt"
rustc -Vv >"$temporary/rustc.txt"
uname -srm >"$temporary/uname.txt"
LC_ALL=C lscpu | sed -n \
    -e '/^Architecture:/p' \
    -e '/^Byte Order:/p' \
    -e '/^CPU(s):/p' \
    -e '/^On-line CPU(s) list:/p' \
    -e '/^Model name:/p' \
    -e '/^Vendor ID:/p' \
    -e '/^CPU family:/p' \
    -e '/^Model:/p' \
    -e '/^Thread(s) per core:/p' \
    -e '/^Core(s) per socket:/p' \
    -e '/^Socket(s):/p' \
    -e '/^Flags:/p' \
    >"$temporary/cpu.txt"
for field in isa uarch mvendorid marchid mimpid; do
    printf '%s: %s\n' "$field" "$(cpuinfo_value "$field")" >>"$temporary/cpu.txt"
done

cat >"$temporary/MANIFEST.txt" <<EOF
schema=base64-ng-rvv-native-admission-v1
source_commit=$source_commit
source_status=clean
host=$host
execution_environment=real-hardware
admission_scope=linux-rvv-1.0-vlen256-spacemit-x60
mvendorid=$mvendorid
marchid=$marchid
mimpid=$mimpid
vector_length_bits=256
samples_per_cell=$samples
target_bytes_per_sample=$target_bytes
automatic_minimum_input=192
median_minimum_ratio=1.02
one_sided_sign_test_maximum_p=0.05
signal_context=pass
thread_context=pass
ffi_abi=pass
register_cleanup=pass
EOF

(
    cd "$temporary"
    sha256sum \
        MANIFEST.txt asm-attributes.txt asm-disassembly.txt asm-manifest.txt \
        correctness.txt cpu.txt rustc.txt rvv.csv uname.txt >CHECKSUMS.sha256
)

scripts/validate-rvv-admission-bundle.py "$temporary"
mkdir -p "$(dirname "$output_dir")"
mv "$temporary" "$output_dir"
trap - EXIT INT TERM
echo "RVV admission capture: wrote $output_dir"
