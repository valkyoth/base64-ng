#!/usr/bin/env sh
set -eu

if [ ! -d perf ]; then
    echo "perf checks: skipping; perf/ is not present"
    exit 0
fi

source_commit=""

verify_source_unchanged() {
    current_commit="$(git rev-parse 'HEAD^{commit}')"
    current_status="$(git status --porcelain=v1 --untracked-files=all)"
    if [ "$current_commit" != "$source_commit" ] || [ -n "$current_status" ]; then
        echo "perf checks: source changed during performance campaign" >&2
        exit 1
    fi
}

if [ "${BASE64_NG_RUN_PERF:-0}" = "1" ]; then
    source_commit="$(git rev-parse 'HEAD^{commit}')"
    verify_source_unchanged
    export BASE64_NG_PERF_SOURCE_COMMIT="$source_commit"
fi

perf_rustflags="${RUSTFLAGS:-}"
if [ -n "$perf_rustflags" ]; then
    perf_rustflags="$perf_rustflags --cfg base64_ng_perf_evidence"
else
    perf_rustflags="--cfg base64_ng_perf_evidence"
fi

run_perf() {
    RUSTFLAGS="$perf_rustflags" cargo "$@"
}

echo "perf checks: compile exact-backend benchmark harness"
run_perf check --manifest-path perf/Cargo.toml --bins
run_perf check --manifest-path perf/Cargo.toml --bins --no-default-features
run_perf test --manifest-path perf/Cargo.toml --bins

echo "perf checks: evidence validator mutations"
scripts/test-perf-evidence.py
scripts/test-neon-performance.py

echo "perf checks: correctness before evidence"
run_perf run --quiet --release --manifest-path perf/Cargo.toml -- correctness
run_perf run --quiet --release --manifest-path perf/Cargo.toml --no-default-features -- correctness

echo "perf checks: RustSec advisories"
cargo audit --file perf/Cargo.lock

echo "perf checks: dependency policy"
scripts/cargo-deny-check.sh perf/Cargo.toml perf/deny.toml

for baseline in performance-baselines/commit-*; do
    if [ ! -d "$baseline" ]; then
        continue
    fi
    echo "perf checks: validate retained baseline $baseline"
    for artifact in \
        environment.json \
        availability.csv \
        raw-run-1.csv \
        raw-run-2.csv \
        resources-default.csv \
        resources-no-simd.csv \
        summary.csv \
        admission.csv \
        binary-resources.csv \
        MANIFEST.txt
    do
        if [ ! -s "$baseline/$artifact" ]; then
            echo "perf checks: incomplete retained baseline: $baseline/$artifact" >&2
            exit 1
        fi
    done
    scripts/validate_perf_evidence.py validate \
        "$baseline/raw-run-1.csv" \
        "$baseline/availability.csv" \
        "$baseline/resources-default.csv" \
        "$baseline/environment.json" \
        --expected-run-id run-1 \
        --expected-feature-set simd
    scripts/validate_perf_evidence.py validate \
        "$baseline/raw-run-2.csv" \
        "$baseline/availability.csv" \
        "$baseline/resources-no-simd.csv" \
        "$baseline/environment.json" \
        --expected-run-id run-2 \
        --expected-feature-set no-simd
    scripts/validate_perf_evidence.py compare "$baseline"
    scripts/validate_perf_evidence.py validate-derived "$baseline"
done

if [ "${BASE64_NG_RUN_PERF:-0}" != "1" ]; then
    echo "perf checks: benchmark run skipped; set BASE64_NG_RUN_PERF=1 to execute it"
    exit 0
fi

evidence_dir="${BASE64_NG_PERF_EVIDENCE_DIR:-target/release-evidence/perf}"
campaign_id="${BASE64_NG_PERF_CAMPAIGN_ID:-$(date -u +%Y%m%dT%H%M%SZ)}"
samples="${BASE64_NG_PERF_SAMPLES:-5}"
target_bytes="${BASE64_NG_PERF_TARGET_BYTES:-4194304}"
mkdir -p "$evidence_dir"

export BASE64_NG_PERF_CAMPAIGN_ID="$campaign_id"
export BASE64_NG_PERF_SAMPLES="$samples"
export BASE64_NG_PERF_TARGET_BYTES="$target_bytes"

echo "perf checks: capture environment and backend inventory"
RUSTFLAGS="$perf_rustflags" scripts/capture_perf_environment.py "$evidence_dir/environment.json"
run_perf run --quiet --release --manifest-path perf/Cargo.toml -- availability \
    >"$evidence_dir/availability.csv"
BASE64_NG_PERF_FEATURE_SET=simd run_perf run --quiet --release --manifest-path perf/Cargo.toml -- resources \
    >"$evidence_dir/resources-default.csv"
BASE64_NG_PERF_FEATURE_SET=no-simd run_perf run --quiet --release --manifest-path perf/Cargo.toml --no-default-features -- resources \
    >"$evidence_dir/resources-no-simd.csv"

echo "perf checks: run reproducibility campaign 1"
BASE64_NG_PERF_RUN_ID=run-1 run_perf run --quiet --release \
    --manifest-path perf/Cargo.toml -- benchmark >"$evidence_dir/raw-run-1.csv"

echo "perf checks: run reproducibility campaign 2"
BASE64_NG_PERF_RUN_ID=run-2 run_perf run --quiet --release \
    --manifest-path perf/Cargo.toml -- benchmark >"$evidence_dir/raw-run-2.csv"
verify_source_unchanged

echo "perf checks: validate and summarize evidence"
scripts/validate_perf_evidence.py validate \
    "$evidence_dir/raw-run-1.csv" \
    "$evidence_dir/availability.csv" \
    "$evidence_dir/resources-default.csv" \
    "$evidence_dir/environment.json" \
    --expected-run-id run-1 \
    --expected-feature-set simd
scripts/validate_perf_evidence.py validate \
    "$evidence_dir/raw-run-2.csv" \
    "$evidence_dir/availability.csv" \
    "$evidence_dir/resources-no-simd.csv" \
    "$evidence_dir/environment.json" \
    --expected-run-id run-2 \
    --expected-feature-set no-simd
scripts/validate_perf_evidence.py compare "$evidence_dir"
scripts/validate_perf_evidence.py summarize "$evidence_dir" \
    >"$evidence_dir/summary.csv"
scripts/validate_perf_evidence.py admission "$evidence_dir" \
    >"$evidence_dir/admission.csv"

echo "perf checks: binary size and monomorphization evidence"
printf '%s\n' \
    "schema_version,feature_set,binary_bytes,base64_ng_symbol_count,method" \
    >"$evidence_dir/binary-resources.csv"
for feature_set in default no-default-features simd secrets checked-backend; do
    target_dir="target/release-evidence/perf-build-$feature_set"
    case "$feature_set" in
        default)
            cargo build --quiet --release --target-dir "$target_dir"
            ;;
        no-default-features)
            cargo build --quiet --release --no-default-features --target-dir "$target_dir"
            ;;
        simd)
            cargo build --quiet --release --features simd --target-dir "$target_dir"
            ;;
        secrets)
            cargo build --quiet --release --features secrets --target-dir "$target_dir"
            ;;
        checked-backend)
            cargo build --quiet --release --features checked-backend --target-dir "$target_dir"
            ;;
    esac
    library="$(find "$target_dir/release/deps" -name 'libbase64_ng-*.rlib' -print -quit)"
    bytes="$(wc -c <"$library" | tr -d ' ')"
    symbols="$(nm -C "$library" 2>/dev/null | grep -c 'base64_ng::' || true)"
    printf '1,%s,%s,%s,nm-and-file-size\n' \
        "$feature_set" "$bytes" "$symbols" >>"$evidence_dir/binary-resources.csv"
done

echo "perf checks: correctness after evidence"
run_perf run --quiet --release --manifest-path perf/Cargo.toml -- correctness
verify_source_unchanged

manifest_tmp="$evidence_dir/MANIFEST.txt.tmp"
verify_source_unchanged
{
    echo "base64-ng performance evidence schema 1"
    echo "source_commit=$source_commit"
    echo "source_status=clean"
    echo "campaign_id=$campaign_id"
    echo "sample_count=$samples"
    echo "target_bytes_per_sample=$target_bytes"
    echo "comparison_crates=base64=0.23.0,base64ct=1.8.3"
    echo "minimum_backend_ratio_to_scalar=0.95"
    echo "reproducibility_ratio_range=0.50..2.00"
    echo "stack_bound_method=source constants and size_of; not a dynamic call-stack measurement"
    echo "artifacts:"
    sha256sum "$evidence_dir"/*.csv "$evidence_dir/environment.json"
} >"$manifest_tmp"
verify_source_unchanged
mv "$manifest_tmp" "$evidence_dir/MANIFEST.txt"
verify_source_unchanged
scripts/validate_perf_evidence.py validate-derived "$evidence_dir"
verify_source_unchanged

echo "perf checks: wrote $evidence_dir"
