#!/usr/bin/env sh
set -eu

if [ ! -d perf ]; then
    echo "perf checks: skipping; perf/ is not present"
    exit 0
fi

echo "perf checks: compile benchmark harness"
cargo check --manifest-path perf/Cargo.toml --bins
cargo check --manifest-path perf/Cargo.toml --bins --no-default-features

echo "perf checks: RustSec advisories"
cargo audit --file perf/Cargo.lock

echo "perf checks: dependency policy"
cargo deny --manifest-path perf/Cargo.toml check --config perf/deny.toml

if [ "${BASE64_NG_RUN_PERF:-0}" = "1" ]; then
    evidence_dir="target/release-evidence/perf"
    output_file="$evidence_dir/perf-output.csv"
    scalar_output_file="$evidence_dir/perf-scalar-output.csv"
    manifest="$evidence_dir/MANIFEST.txt"
    command_line="cargo run --release --manifest-path perf/Cargo.toml"
    scalar_command_line="cargo run --release --manifest-path perf/Cargo.toml --no-default-features"

    echo "perf checks: run scalar benchmark harness"
    mkdir -p "$evidence_dir"

    scalar_status=0
    cargo run --release --manifest-path perf/Cargo.toml --no-default-features >"$scalar_output_file" 2>&1 || scalar_status="$?"
    cat "$scalar_output_file"

    echo "perf checks: run default benchmark harness"
    status=0
    cargo run --release --manifest-path perf/Cargo.toml >"$output_file" 2>&1 || status="$?"
    cat "$output_file"

    {
        echo "base64-ng performance evidence"
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
        echo "$scalar_command_line"
        echo
        echo "status:"
        echo "$status"
        echo "scalar_status:"
        echo "$scalar_status"
        echo
        echo "artifacts:"
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum "$output_file" "$scalar_output_file"
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 "$output_file" "$scalar_output_file"
        else
            cksum "$output_file" "$scalar_output_file"
        fi
        echo
        echo "interpretation:"
        echo "This is local benchmark evidence for this machine and command only."
        echo "perf-scalar-output.csv is built with perf --no-default-features, which disables base64-ng SIMD."
        echo "perf-output.csv is built with perf default features, which enables base64-ng SIMD and records the active backend in each row."
        echo "Performance numbers are release notes evidence only when paired with hardware, OS, Rust version, CPU governor, and exact command output."
    } >"$manifest"

    echo "perf checks: wrote $evidence_dir"

    if [ "$status" -ne 0 ]; then
        exit "$status"
    fi
    if [ "$scalar_status" -ne 0 ]; then
        exit "$scalar_status"
    fi
else
    echo "perf checks: benchmark run skipped; set BASE64_NG_RUN_PERF=1 to execute it"
fi
