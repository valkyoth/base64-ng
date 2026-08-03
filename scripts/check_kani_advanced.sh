#!/usr/bin/env sh
set -eu

if [ ! -d kani ]; then
    echo "Advanced Kani checks: skipping; kani/ is not present"
    exit 0
fi

scripts/validate-kani-proof-inventory.py

kani_toolchain="${BASE64_NG_KANI_TOOLCHAIN:-1.90.0-x86_64-unknown-linux-gnu}"
kani_timeout="${BASE64_NG_KANI_ADVANCED_TIMEOUT:-10m}"
kani_memory_kib="${BASE64_NG_KANI_ADVANCED_MEMORY_KIB:-16777216}"
evidence="target/release-evidence/kani/advanced"
mkdir -p "$evidence"
awk -F '\t' 'NR == 1 || $2 == "advanced"' kani/harnesses.tsv >"$evidence/harnesses.tsv"
rm -f \
    "$evidence/commands.txt" \
    "$evidence/SHA256SUMS" \
    "$evidence"/result-*.txt \
    "$evidence"/resources-*.txt \
    "$evidence/status.txt" \
    "$evidence/unsupported-constructs.txt"

if ! rustup toolchain list | grep -q "^$kani_toolchain"; then
    echo "Advanced Kani checks: skipping; Rust toolchain $kani_toolchain is not installed"
    exit 0
fi

cargo_kani() {
    rustup run "$kani_toolchain" cargo kani "$@"
}

if ! cargo_kani --version >/dev/null 2>&1; then
    echo "Advanced Kani checks: skipping; cargo kani is not installed"
    exit 0
fi

if [ -n "${RUSTFLAGS:-}" ]; then
    export RUSTFLAGS="$RUSTFLAGS --cfg base64_ng_kani_advanced"
else
    export RUSTFLAGS="--cfg base64_ng_kani_advanced"
fi

rustup run "$kani_toolchain" rustc -Vv >"$evidence/rustc.txt"
cargo_kani --version >"$evidence/kani-version.txt"

apply_memory_limit() {
    if ! ulimit -v "$kani_memory_kib" 2>/dev/null; then
        echo "Advanced Kani checks: warning: virtual-memory limit unavailable" >&2
    fi
}

print_summary() {
    grep -E \
        '^(warning: Found|warning: Kani|Checking harness |SUMMARY:|VERIFICATION:|Manual Harness Summary:|Complete -|Verification failed for -)' \
        "$1" || true
}

run_kani() {
    label="$1"
    shift
    log="$(mktemp)"
    resource="$evidence/resources-$label.txt"

    echo "Advanced Kani checks: running $label"
    printf 'rustup run %s cargo kani' "$kani_toolchain" >>"$evidence/commands.txt"
    for argument in "$@"; do
        printf ' %s' "$argument" >>"$evidence/commands.txt"
    done
    printf '\n' >>"$evidence/commands.txt"

    if /usr/bin/time --version >/dev/null 2>&1; then
        if (
            apply_memory_limit
            exec /usr/bin/time -v -o "$resource" \
                rustup run "$kani_toolchain" cargo kani "$@"
        ) >"$log" 2>&1; then
            status=0
        else
            status="$?"
        fi
    elif (
        apply_memory_limit
        printf '%s\n' "peak-memory unavailable: GNU time not installed" >"$resource"
        exec rustup run "$kani_toolchain" cargo kani "$@"
    ) >"$log" 2>&1; then
        status=0
    else
        status="$?"
    fi

    cp "$log" "$evidence/result-$label.txt"
    if [ "$status" -eq 0 ]; then
        print_summary "$log"
        rm -f "$log"
        return 0
    fi

    if grep -q "Kani Rust Verifier" "$log" && grep -q "requires rustc" "$log"; then
        rm -f "$log"
        echo "Advanced Kani checks: skipping; installed Kani compiler is older than this crate's rust-version"
        printf '%s\n' SKIP >"$evidence/status.txt"
        exit 0
    fi

    grep -n -E 'Status: FAILURE|VERIFICATION:- FAILED|Verification failed for -|timed out|Out of memory' \
        "$log" || true
    tail -n 200 "$log"
    rm -f "$log"
    printf '%s\n' FAIL >"$evidence/status.txt"
    exit "$status"
}

run_harness() {
    harness="$1"
    run_kani \
        "$harness" \
        --no-default-features \
        --features secrets \
        --harness "$harness" \
        -Z unstable-options \
        --harness-timeout "$kani_timeout"
}

run_manifest_advanced() {
    tail -n +2 kani/harnesses.tsv | while IFS="$(printf '\t')" read -r harness set _rest; do
        if [ "$set" = "advanced" ]; then
            run_harness "$harness"
        fi
    done
}

echo "Advanced Kani checks: using Rust toolchain $kani_toolchain"
run_kani \
    "advanced-codegen" \
    --no-default-features \
    --features secrets \
    --only-codegen

if [ "${BASE64_NG_KANI_ALL_ADVANCED:-0}" = "1" ]; then
    run_manifest_advanced
else
    if [ "${BASE64_NG_KANI_PROVE_FINAL_CORE:-0}" = "1" ]; then
        awk -F '\t' '$2 == "advanced" && $3 == "src/kani_v2_core_proofs.rs" { print $1 }' \
            kani/harnesses.tsv | while IFS= read -r harness; do run_harness "$harness"; done
    else
        echo "Advanced Kani checks: skipped final-core RFC refinement proofs"
        echo "Advanced Kani checks: set BASE64_NG_KANI_PROVE_FINAL_CORE=1 to run them"
    fi

    if [ "${BASE64_NG_KANI_PROVE_ASSURANCE:-0}" = "1" ]; then
        awk -F '\t' '$2 == "advanced" && $3 == "src/kani_assurance_proofs.rs" { print $1 }' \
            kani/harnesses.tsv | while IFS= read -r harness; do run_harness "$harness"; done
    else
        echo "Advanced Kani checks: skipped assurance protocol-model proofs"
        echo "Advanced Kani checks: set BASE64_NG_KANI_PROVE_ASSURANCE=1 to run them"
    fi

    if [ "${BASE64_NG_KANI_PROVE_SECRET_FRAMES:-0}" = "1" ]; then
        awk -F '\t' '$2 == "advanced" && $3 == "src/kani_secret_proofs.rs" { print $1 }' \
            kani/harnesses.tsv | while IFS= read -r harness; do run_harness "$harness"; done
    else
        echo "Advanced Kani checks: skipped secret-frame proofs"
        echo "Advanced Kani checks: set BASE64_NG_KANI_PROVE_SECRET_FRAMES=1 to run them"
    fi

    if [ "${BASE64_NG_KANI_PROVE_SECRET_ENCODING:-0}" = "1" ]; then
        awk -F '\t' '$2 == "advanced" && $3 == "src/kani_secret_encode_proofs.rs" { print $1 }' \
            kani/harnesses.tsv | while IFS= read -r harness; do run_harness "$harness"; done
    else
        echo "Advanced Kani checks: skipped secret-encoder proofs"
        echo "Advanced Kani checks: set BASE64_NG_KANI_PROVE_SECRET_ENCODING=1 to run them"
    fi

    if [ "${BASE64_NG_KANI_PROVE_PUBLIC_SURFACE:-0}" = "1" ]; then
        run_harness advanced_public_strict_decode_surfaces_do_not_panic_for_bounded_inputs
    else
        echo "Advanced Kani checks: skipped public-surface proof"
        echo "Advanced Kani checks: set BASE64_NG_KANI_PROVE_PUBLIC_SURFACE=1 to run it"
    fi
fi

if [ "${BASE64_NG_KANI_EXPENSIVE_WRAPPED:-0}" = "1" ]; then
    echo "Advanced Kani checks: running exploratory high-cost proofs"
    awk -F '\t' '$2 == "exploratory" { print $1 }' kani/harnesses.tsv \
        | while IFS= read -r harness; do run_harness "$harness"; done
else
    echo "Advanced Kani checks: skipped exploratory high-cost proofs"
    echo "Advanced Kani checks: set BASE64_NG_KANI_EXPENSIVE_WRAPPED=1 to run them"
fi

if grep -h -A8 -E 'unsupported constructs|does not support concurrency' \
    "$evidence"/result-*.txt >"$evidence/unsupported-constructs.txt"; then
    :
else
    printf '%s\n' none >"$evidence/unsupported-constructs.txt"
fi
printf '%s\n' PASS >"$evidence/status.txt"
sha256sum "$evidence"/*.txt "$evidence/harnesses.tsv" >"$evidence/SHA256SUMS"
echo "Advanced Kani checks: evidence written to $evidence"
