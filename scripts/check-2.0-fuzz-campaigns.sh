#!/usr/bin/env sh
set -eu

targets="$(cat scripts/fuzz-release-targets.txt)"

echo "2.0 fuzz campaigns: complete target inventory"
grep -F -q 'scripts/fuzz-release-targets.txt' scripts/check_fuzz.sh
grep -F -q 'scripts/fuzz-release-targets.txt' scripts/check_fuzz_corpus.sh
for target in $targets; do
    grep -F -q "name = \"$target\"" fuzz/Cargo.toml
    test -s "fuzz/fuzz_targets/$target.rs"
done

echo "2.0 fuzz campaigns: target compilation"
cargo check --manifest-path fuzz/Cargo.toml --bins

echo "2.0 fuzz campaigns: deterministic runtime and partition properties"
cargo test --all-features --test v2_fuzz_properties

echo "2.0 fuzz campaigns: protected teardown and unsafe-provider isolation"
cargo test --all-features --test v2_assurance
grep -F -q 'BASE64_NG_TEST_SUBPROCESS_RUNNER' tests/v2_assurance.rs
for cross_gate in \
    scripts/check_sve_qemu.sh \
    scripts/check_riscv_qemu.sh \
    scripts/check_big_endian_qemu.sh
do
    grep -F -q 'BASE64_NG_TEST_SUBPROCESS_RUNNER' "$cross_gate"
done

echo "2.0 fuzz campaigns: async panic, cancellation, and backpressure regressions"
cargo test -p base64-ng-tokio \
    --test tokio_reader_adversarial \
    --test tokio_writer_adversarial

echo "2.0 fuzz campaigns: integration callback panic regressions"
cargo test -p base64-ng-bytes --test bytes downstream_panic
cargo test -p base64-ng-serde --all-features --test serializer_modes \
    secret_serialization_resumes_serializer_panics_through_the_cleanup_guard

echo "2.0 fuzz campaigns: static no_std token boundary"
grep -F -q 'StaticBackendToken::for_compiled_target()' \
    portability/aarch64_static_neon_smoke/src/main.rs
grep -F -q 'StaticBackendToken::assume_supported(backend)' \
    portability/x86_static_encode_smoke/src/main.rs
grep -F -q 'if backend_matches_target(backend)' src/simd/static_token.rs

echo "2.0 fuzz campaigns: evidence contract"
for evidence in \
    '-print_final_stats=1' \
    'corpus-hashes:' \
    'artifacts=' \
    'minimization:' \
    'BASE64_NG_RUN_FUZZ_RELEASE'
do
    grep -F -q -- "$evidence" scripts/check_fuzz.sh
done
for distributed_evidence in \
    'duration_seconds' \
    'source_tree' \
    'architecture_class' \
    'artifact_count' \
    'BASE64_NG_FUZZ_SHARD_DIR'
do
    grep -F -q -- "$distributed_evidence" \
        scripts/capture-fuzz-shard.sh \
        scripts/fuzz_shard_evidence.py \
        scripts/stable_release_gate.sh
done
grep -F -q 'scripts/test-fuzz-shard-evidence.py' scripts/checks.sh

echo "2.0 fuzz campaigns: complete adversarial target and property evidence ok"
