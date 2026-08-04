#!/usr/bin/env sh
set -eu

document="docs/2.0_MEMORY_SANITIZER_HARDWARE_EVIDENCE.md"
evidence_dir="target/release-evidence/commit-53"
manifest="$evidence_dir/MANIFEST.txt"
mkdir -p "$evidence_dir"

for required in \
    'NEON performance | native evidence required before 2.0.0' \
    'RVV and SVE remain non-dispatchable candidates' \
    'Base 2.0 ships no persistent provider' \
    'Abort, `panic=abort`, OOM abort, process termination' \
    'No growth after' \
    'UndefinedBehaviorSanitizer is not a separately supported Rust sanitizer'
do
    if ! grep -F -q "$required" "$document"; then
        echo "2.0 memory/hardware evidence: missing documentation marker: $required" >&2
        exit 1
    fi
done

echo "2.0 memory/hardware evidence: secret allocation identity"
cargo test --all-features --lib \
    'v2::secret_decoder_tests::vector_frame_never_reallocates_after_classified_input' \
    -- --exact
cargo test --all-features --lib \
    'v2::secret_encoder_tests::vector_encoder_never_reallocates_after_classified_input' \
    -- --exact

echo "2.0 memory/hardware evidence: volatile provider and combined limits"
cargo test --all-features --lib \
    'v2::assurance::tests::active_and_quarantined_allocations_share_every_provider_budget' \
    -- --exact
cargo test --all-features --lib \
    'v2::assurance::tests::fresh_volatile_provider_never_recovers_prior_instance_state' \
    -- --exact

echo "2.0 memory/hardware evidence: report schema tests"
scripts/test-big-endian-hardware-evidence.py
scripts/test-riscv-hardware-evidence.py
scripts/test-sve-hardware-evidence.py
scripts/test-perf-evidence.py

echo "2.0 memory/hardware evidence: retained x86 campaign"
scripts/validate-x86-encode-performance.py \
    performance-baselines/dispatch-commit-34-amd-9950x3d-linux/x86-encode.csv
scripts/validate-x86-decode-performance.py \
    performance-baselines/dispatch-commit-34-amd-9950x3d-linux/x86-decode.csv

neon_apple="performance-baselines/dispatch-2.0-neon-apple-silicon"
neon_linux="performance-baselines/dispatch-2.0-neon-aarch64-linux"
neon_status="pending-native-performance"
if [ -f "$neon_apple/MANIFEST.txt" ] && [ -f "$neon_linux/MANIFEST.txt" ]; then
    neon_status="retained-native-performance"
fi
if [ "${BASE64_NG_REQUIRE_COMMIT53_NATIVE:-0}" = "1" ] && \
    [ "$neon_status" != "retained-native-performance" ]; then
    echo "2.0 memory/hardware evidence: release requires retained Apple and Linux NEON bundles" >&2
    exit 1
fi

{
    echo "base64-ng Commit 53 evidence inventory"
    echo "source_commit=$(git rev-parse HEAD)"
    echo "rustc=$(rustc -V)"
    echo "cargo=$(cargo -V)"
    echo "host=$(rustc -vV | sed -n 's/^host: //p')"
    echo "x86_automatic_dispatch=retained-native-correctness-and-performance"
    echo "neon_automatic_dispatch=$neon_status"
    echo "wasm_simd128=runtime-and-browser-evidence-not-hardware-attestation"
    echo "big_endian=scalar-qemu-portability-native-report-optional"
    echo "rvv=candidate-qemu-only-not-dispatchable"
    echo "sve=candidate-qemu-only-not-dispatchable"
    echo "persistent_provider=none"
    echo "miri=separate-release-artifact"
    echo "address_leak_thread_sanitizers=separate-release-artifacts"
    echo "semantic_corpus=semantic-corpus/v1/cases.tsv"
    echo "resource_schema=scripts/perf_evidence_schema.py"
} >"$manifest"

echo "2.0 memory/hardware evidence: wrote $manifest"
echo "2.0 memory/hardware evidence: ok ($neon_status)"
