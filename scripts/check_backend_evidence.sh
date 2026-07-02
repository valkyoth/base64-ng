#!/usr/bin/env sh
set -eu

evidence_dir="target/release-evidence/backend"
runtime_output="$evidence_dir/runtime-backend-report.txt"
prototype_output="$evidence_dir/simd-prototype-equivalence.txt"
manifest="$evidence_dir/MANIFEST.txt"

mkdir -p "$evidence_dir"

echo "backend evidence: runtime report"
runtime_status=0
cargo test --test rfc4648 --all-features runtime_backend_report_matches_admission_state -- --nocapture >"$runtime_output" 2>&1 || runtime_status="$?"
cat "$runtime_output"

echo "backend evidence: SIMD prototype scalar-equivalence tests"
prototype_status=0
cargo test --all-features simd:: -- --nocapture >"$prototype_output" 2>&1 || prototype_status="$?"
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
    echo "cargo test --test rfc4648 --all-features runtime_backend_report_matches_admission_state -- --nocapture"
    echo "cargo test --all-features simd:: -- --nocapture"
    echo
    echo "status:"
    echo "runtime_backend_report=$runtime_status"
    echo "simd_prototype_equivalence=$prototype_status"
    echo "runtime_dispatch=avx512-vbmi-encode-then-avx2-encode-then-ssse3-sse4.1-encode-on-x86-or-neon-encode-on-aarch64-when-supported-else-scalar"
    echo "active_backend_admitted=avx512-vbmi-or-avx2-or-ssse3-sse4.1-or-neon-encode"
    echo "prototype_state=real-non-dispatchable"
    echo "prototype_avx512_vbmi=admitted-encode-backend"
    echo "prototype_avx2=admitted-encode-backend"
    echo "prototype_ssse3_sse41=admitted-encode-backend"
    echo "prototype_ssse3_sse41_decode=real-non-dispatchable"
    echo "prototype_neon=admitted-encode-backend-aarch64-standard-family"
    echo "prototype_wasm_simd128=real-non-dispatchable-compile-evidence-only"
    echo "wasm_simd128_evidence=compile-test-binary-only"
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
    echo "This evidence records runtime backend reporting, admitted AVX-512 VBMI, AVX2, SSSE3/SSE4.1, or NEON encode dispatch when supported, and remaining inactive SIMD prototype scalar-equivalence results for this machine."
    echo "The SSSE3/SSE4.1 decode prototype is fixed-block, non-dispatchable evidence only; public decode APIs still report scalar execution."
    echo "Wasm results are prototype evidence; they are not active backend admission evidence."
    echo "The admitted x86 AVX-512 VBMI, AVX2, and SSSE3/SSE4.1 paths exercise real fixed-block vector encode logic for Standard and URL-safe alphabets when the required CPU feature bundle is available."
    echo "On AArch64 hosts, the admitted NEON path exercises real fixed-block vector encode logic for Standard and URL-safe alphabets; 32-bit ARM remains scaffold evidence."
    echo "Wasm simd128 evidence is produced by scripts/check_simd_feature_bundles.sh as compile/test-binary evidence only."
    echo "It does not admit accelerated dispatch or replace fuzzing, Miri, unsafe inventory review, generated assembly evidence, benchmark evidence, or release notes."
} >"$manifest"

echo "backend evidence: wrote $evidence_dir"

if [ "$runtime_status" -ne 0 ]; then
    exit "$runtime_status"
fi

if [ "$prototype_status" -ne 0 ]; then
    exit "$prototype_status"
fi
