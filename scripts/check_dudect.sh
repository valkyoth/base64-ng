#!/usr/bin/env sh
set -eu

if [ ! -d dudect ]; then
    echo "dudect checks: skipping; dudect/ is not present"
    exit 0
fi

if [ "${BASE64_NG_RUN_DUDECT:-0}" = "1" ]; then
    mkdir -p target/release-evidence/dudect
    rm -f \
        target/release-evidence/dudect/MANIFEST.txt \
        target/release-evidence/dudect/MANIFEST.txt.tmp
fi

echo "dudect checks: compile timing harness"
cargo check --manifest-path dudect/Cargo.toml --bins

echo "dudect checks: RustSec advisories"
cargo audit --file dudect/Cargo.lock

echo "dudect checks: dependency policy"
scripts/cargo-deny-check.sh dudect/Cargo.toml dudect/deny.toml

if [ "${BASE64_NG_RUN_DUDECT:-0}" = "1" ]; then
    . scripts/evidence-source.sh
    evidence_capture_source "dudect timing evidence"

    evidence_dir="target/release-evidence/dudect"
    output_file="$evidence_dir/dudect-output.txt"
    manifest="$evidence_dir/MANIFEST.txt"
    manifest_tmp="$evidence_dir/MANIFEST.txt.tmp"
    samples="${BASE64_NG_DUDECT_SAMPLES:-20000}"
    iterations="${BASE64_NG_DUDECT_ITERS:-64}"
    threshold="${BASE64_NG_DUDECT_THRESHOLD:-10}"
    warmup="${BASE64_NG_DUDECT_WARMUP:-1000}"
    if [ "${BASE64_NG_DUDECT_RELEASE:-0}" = "1" ]; then
        for value in "$samples" "$iterations" "$warmup"; do
            case "$value" in
                '' | *[!0-9]*)
                    echo "dudect checks: release parameters must be integers" >&2
                    exit 1
                    ;;
            esac
        done
        if [ "$samples" -lt 20000 ] || [ "$iterations" -lt 64 ] || [ "$warmup" -lt 1000 ]; then
            echo "dudect checks: release evidence requires at least 20000 samples, 64 iterations, and 1000 warmups" >&2
            exit 1
        fi
    fi
    command_line="cargo run --release --manifest-path dudect/Cargo.toml -- --samples $samples --iters $iterations --threshold $threshold --warmup $warmup"

    echo "dudect checks: run timing harness"
    mkdir -p "$evidence_dir"

    cleanup_manifest() {
        rm -f "$manifest_tmp"
    }
    trap cleanup_manifest EXIT INT TERM

    status=0
    cargo run --release --manifest-path dudect/Cargo.toml -- \
        --samples "$samples" \
        --iters "$iterations" \
        --threshold "$threshold" \
        --warmup "$warmup" >"$output_file" 2>&1 || status="$?"

    cat "$output_file"
    evidence_verify_source "dudect timing evidence"

    if [ "$status" -ne 0 ]; then
        exit "$status"
    fi

    {
        echo "base64-ng dudect-style timing evidence"
        echo
        evidence_write_source_manifest
        echo
        echo "rustc:"
        rustc -Vv
        echo
        echo "cargo:"
        cargo -V
        echo
        echo "system:"
        if command -v uname >/dev/null 2>&1; then
            uname -a
        else
            echo "uname unavailable"
        fi
        echo
        echo "processor:"
        if command -v lscpu >/dev/null 2>&1; then
            lscpu
        elif command -v sysctl >/dev/null 2>&1; then
            sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "processor identity unavailable"
        else
            echo "processor identity unavailable"
        fi
        echo
        echo "build boundary:"
        echo "target=$(rustc -vV | sed -n 's/^host: //p')"
        echo "profile=release"
        echo "features=secrets,simd (secret states remain scalar)"
        echo "RUSTFLAGS=${RUSTFLAGS:-<unset>}"
        echo
        echo "command:"
        echo "$command_line"
        echo
        echo "parameters:"
        echo "samples=$samples"
        echo "iterations=$iterations"
        echo "threshold=$threshold"
        echo "warmup=$warmup"
        echo "status=$status"
        echo
        echo "artifacts:"
        evidence_checksum_file "$output_file"
        echo
        echo "interpretation:"
        echo "This is empirical 2.0 secret-frame timing evidence for this binary and machine only."
        echo "Equal-work cases compare valid contents, malformed positions/classes, the fixed-work pre-gate core, encode mappings, and equality mismatch positions."
        echo "Public-length decode, encode, and equality cases are informational and may differ."
        echo "Whole-call valid/invalid equality is not claimed because success performs a post-gate release copy."
        echo "Ordinary SIMD is compiled into the binary, but reviewed secret states remain scalar and do not dispatch to it."
        echo "It does not replace generated-code review, Kani, Miri, fuzzing, or deterministic tests."
    } >"$manifest_tmp"

    mv "$manifest_tmp" "$manifest"
    trap - EXIT INT TERM

    echo "dudect checks: wrote $evidence_dir"

else
    echo "dudect checks: timing run skipped; set BASE64_NG_RUN_DUDECT=1 to execute it"
fi
