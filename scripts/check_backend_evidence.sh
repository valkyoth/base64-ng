#!/usr/bin/env sh
set -eu

evidence_dir="target/release-evidence/backend"
runtime_output="$evidence_dir/runtime-backend-report.txt"
prototype_output="$evidence_dir/simd-prototype-equivalence.txt"
manifest="$evidence_dir/MANIFEST.txt"

mkdir -p "$evidence_dir"

echo "backend evidence: runtime report"
runtime_status=0
cargo test --test rfc4648 --all-features runtime_backend_report_keeps_scalar_active -- --nocapture >"$runtime_output" 2>&1 || runtime_status="$?"
cat "$runtime_output"

echo "backend evidence: SIMD prototype scalar-equivalence tests"
prototype_status=0
cargo test --all-features simd::tests:: -- --nocapture >"$prototype_output" 2>&1 || prototype_status="$?"
cat "$prototype_output"

{
    echo "base64-ng backend evidence"
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
    echo "commands:"
    echo "cargo test --test rfc4648 --all-features runtime_backend_report_keeps_scalar_active -- --nocapture"
    echo "cargo test --all-features simd::tests:: -- --nocapture"
    echo
    echo "status:"
    echo "runtime_backend_report=$runtime_status"
    echo "simd_prototype_equivalence=$prototype_status"
    echo
    echo "artifacts:"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$runtime_output" "$prototype_output"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$runtime_output" "$prototype_output"
    else
        cksum "$runtime_output" "$prototype_output"
    fi
    echo
    echo "interpretation:"
    echo "This evidence records runtime backend reporting and inactive SIMD prototype scalar-equivalence results for this machine."
    echo "It does not admit accelerated dispatch or replace fuzzing, Miri, unsafe inventory review, benchmark evidence, or release notes."
} >"$manifest"

echo "backend evidence: wrote $evidence_dir"

if [ "$runtime_status" -ne 0 ]; then
    exit "$runtime_status"
fi

if [ "$prototype_status" -ne 0 ]; then
    exit "$prototype_status"
fi
