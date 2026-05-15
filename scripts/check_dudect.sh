#!/usr/bin/env sh
set -eu

if [ ! -d dudect ]; then
    echo "dudect checks: skipping; dudect/ is not present"
    exit 0
fi

echo "dudect checks: compile timing harness"
cargo check --manifest-path dudect/Cargo.toml --bins

echo "dudect checks: RustSec advisories"
cargo audit --file dudect/Cargo.lock

echo "dudect checks: dependency policy"
cargo deny --manifest-path dudect/Cargo.toml check --config dudect/deny.toml

if [ "${BASE64_NG_RUN_DUDECT:-0}" = "1" ]; then
    evidence_dir="target/release-evidence/dudect"
    output_file="$evidence_dir/dudect-output.txt"
    manifest="$evidence_dir/MANIFEST.txt"
    samples="${BASE64_NG_DUDECT_SAMPLES:-20000}"
    iterations="${BASE64_NG_DUDECT_ITERS:-64}"
    threshold="${BASE64_NG_DUDECT_THRESHOLD:-10}"
    warmup="${BASE64_NG_DUDECT_WARMUP:-1000}"
    command_line="cargo run --release --manifest-path dudect/Cargo.toml -- --samples $samples --iters $iterations --threshold $threshold --warmup $warmup"

    echo "dudect checks: run timing harness"
    mkdir -p "$evidence_dir"

    status=0
    cargo run --release --manifest-path dudect/Cargo.toml -- \
        --samples "$samples" \
        --iters "$iterations" \
        --threshold "$threshold" \
        --warmup "$warmup" >"$output_file" 2>&1 || status="$?"

    cat "$output_file"

    {
        echo "base64-ng dudect-style timing evidence"
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
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum "$output_file"
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 "$output_file"
        else
            cksum "$output_file"
        fi
        echo
        echo "interpretation:"
        echo "This is empirical timing evidence for this binary and machine only."
        echo "It does not replace generated-code review, Kani, Miri, fuzzing, or deterministic tests."
    } >"$manifest"

    echo "dudect checks: wrote $evidence_dir"

    if [ "$status" -ne 0 ]; then
        exit "$status"
    fi
else
    echo "dudect checks: timing run skipped; set BASE64_NG_RUN_DUDECT=1 to execute it"
fi
