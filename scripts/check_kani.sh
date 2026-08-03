#!/usr/bin/env sh
set -eu

if [ ! -d kani ]; then
    echo "Kani checks: skipping; kani/ is not present"
    exit 0
fi

scripts/validate-kani-proof-inventory.py

kani_toolchain="${BASE64_NG_KANI_TOOLCHAIN:-1.90.0-x86_64-unknown-linux-gnu}"
kani_timeout="${BASE64_NG_KANI_TIMEOUT:-5m}"
kani_memory_kib="${BASE64_NG_KANI_MEMORY_KIB:-8388608}"
evidence="target/release-evidence/kani/normal"
mkdir -p "$evidence"
rm -f "$evidence/SHA256SUMS"
awk -F '\t' 'NR == 1 || $2 == "normal"' kani/harnesses.tsv >"$evidence/harnesses.tsv"

if ! rustup toolchain list | grep -q "^$kani_toolchain"; then
    echo "Kani checks: skipping; Rust toolchain $kani_toolchain is not installed"
    exit 0
fi

cargo_kani() {
    rustup run "$kani_toolchain" cargo kani "$@"
}

if ! cargo_kani --version >/dev/null 2>&1; then
    echo "Kani checks: skipping; cargo kani is not installed"
    exit 0
fi

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

echo "Kani checks: using Rust toolchain $kani_toolchain"
rustup run "$kani_toolchain" rustc -Vv >"$evidence/rustc.txt"
cargo_kani --version >"$evidence/kani-version.txt"
printf '%s\n' \
    "rustup run $kani_toolchain cargo kani --no-default-features -Z unstable-options --harness-timeout $kani_timeout" \
    >"$evidence/command.txt"

run_bounded_kani() {
    if /usr/bin/time --version >/dev/null 2>&1; then
        (
            if ! ulimit -v "$kani_memory_kib" 2>/dev/null; then
                echo "Kani checks: warning: virtual-memory limit unavailable" >&2
            fi
            exec /usr/bin/time -v -o "$evidence/resources.txt" \
                rustup run "$kani_toolchain" cargo kani \
                --no-default-features \
                -Z unstable-options \
                --harness-timeout "$kani_timeout"
        ) >"$log" 2>&1
    else
        printf '%s\n' "peak-memory unavailable: GNU time not installed" >"$evidence/resources.txt"
        (
            if ! ulimit -v "$kani_memory_kib" 2>/dev/null; then
                echo "Kani checks: warning: virtual-memory limit unavailable" >&2
            fi
            exec rustup run "$kani_toolchain" cargo kani \
                --no-default-features \
                -Z unstable-options \
                --harness-timeout "$kani_timeout"
        ) >"$log" 2>&1
    fi
}

print_summary() {
    grep -E \
        '^(warning: Found|warning: Kani|Checking harness |SUMMARY:|VERIFICATION:|Manual Harness Summary:|Complete -|Verification failed for -)' \
        "$1" || true
}

if run_bounded_kani; then
    cp "$log" "$evidence/result.txt"
    print_summary "$log"
    printf '%s\n' PASS >"$evidence/status.txt"
    if grep -A8 -E 'unsupported constructs|does not support concurrency' "$log" \
        >"$evidence/unsupported-constructs.txt"; then
        :
    else
        printf '%s\n' none >"$evidence/unsupported-constructs.txt"
    fi
    sha256sum "$evidence"/*.txt "$evidence/harnesses.tsv" >"$evidence/SHA256SUMS"
    exit 0
else
    status="$?"
fi

if grep -q "Kani Rust Verifier" "$log" && grep -q "requires rustc" "$log"; then
    echo "Kani checks: skipping; installed Kani compiler is older than this crate's rust-version"
    cp "$log" "$evidence/result.txt"
    printf '%s\n' SKIP >"$evidence/status.txt"
    exit 0
fi

cp "$log" "$evidence/result.txt"
grep -n -E 'Status: FAILURE|VERIFICATION:- FAILED|Verification failed for -|timed out|Out of memory' \
    "$log" || true
tail -n 200 "$log"
printf '%s\n' FAIL >"$evidence/status.txt"
exit "$status"
