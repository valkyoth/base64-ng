#!/usr/bin/env sh
set -eu

draft="docs/SIMD_ENCODE_ADMISSION_DRAFT.md"
manifest="docs/SIMD_ADMISSION.md"
simd_doc="docs/SIMD.md"

test -s "$draft"
test -s "$manifest"
test -s "$simd_doc"

for required_text in \
    "It is not an admission record" \
    "AVX2 and SSSE3/SSE4.1 encode are already admitted" \
    "every additional backend or broader API surface remains pending" \
    "Decode acceleration" \
    "x86/x86_64 runtime dispatch only" \
    "Unsupported CPUs must execute scalar code without illegal instructions" \
    "Any backend whose evidence is incomplete stays candidate-only" \
    "full \`Engine::encode_slice\`, \`encode_slice_clear_tail\`, and alloc helper" \
    "undersized-output error parity" \
    "in-place encode parity" \
    "custom alphabet fallback behavior" \
    "fuzz differential evidence" \
    "generated optimized assembly" \
    "hardware evidence from a CPU that actually supports the backend" \
    "benchmark output with scalar baseline" \
    "AArch64 NEON must include generated assembly" \
    "wasm \`simd128\` must include generated-code/JIT evidence" \
    "accelerated_backend_active=true" \
    "security_posture=accelerated" \
    "candidate_detection_mode" \
    "high-assurance scalar policy" \
    "backend:" \
    "target triple:" \
    "CPU model:" \
    "scalar throughput:" \
    "SIMD throughput:" \
    "speedup:" \
    "raw artifact:" \
    "Required precision" \
    "Forbidden wording until proven" \
    "Before changing \`ActiveBackend\`, answer all of these"
do
    if ! grep -F -q "$required_text" "$draft"; then
        echo "simd encode admission draft: missing required text: $required_text" >&2
        exit 1
    fi
done

for required_command in \
    "cargo test --all-features" \
    "cargo clippy --all-features --all-targets -- -D warnings" \
    "scripts/check_simd_feature_bundles.sh" \
    "scripts/check_backend_evidence.sh" \
    "scripts/generate_simd_asm_evidence.sh" \
    "BASE64_NG_RUN_PERF=1 scripts/check_perf.sh"
do
    if ! grep -F -q "$required_command" "$draft"; then
        echo "simd encode admission draft: missing release-candidate command: $required_command" >&2
        exit 1
    fi
done

for policy_doc in "$simd_doc" docs/RELEASE_EVIDENCE.md docs/BENCHMARKS.md
do
    if ! grep -F -q "SIMD_ENCODE_ADMISSION_DRAFT.md" "$policy_doc"; then
        echo "simd encode admission draft: policy doc must link to the draft: $policy_doc" >&2
        exit 1
    fi
done

if ! grep -F -q "Admitted backends: AVX2 encode and SSSE3/SSE4.1 encode" "$manifest"; then
    echo "simd encode admission draft: manifest must report the admitted AVX2 and SSSE3/SSE4.1 encode backends" >&2
    exit 1
fi

if ! grep -F -q "Active backend priority: AVX2, then SSSE3/SSE4.1" "$manifest"; then
    echo "simd encode admission draft: manifest must report AVX2 and SSSE3/SSE4.1 encode priority" >&2
    exit 1
fi

if grep -R -q "ActiveBackend::Avx512\|ActiveBackend::Neon\|ActiveBackend::Wasm\|ActiveBackend::Simd" src; then
    echo "simd encode admission draft: accelerated ActiveBackend variant exists before admission" >&2
    exit 1
fi

echo "simd encode admission draft: ok"
