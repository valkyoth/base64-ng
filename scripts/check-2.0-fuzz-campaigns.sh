#!/usr/bin/env sh
set -eu

targets="
decode
in_place
stream_chunks
differential
profiles
x86_encode
x86_decode
neon
mime_body
pem_document
multibase_family
imap_payload
password_records
openpgp_armor
v2_runtime_codec
v2_incremental
v2_async
v2_assurance
"

echo "2.0 fuzz campaigns: complete target inventory"
for target in $targets; do
    grep -F -q "name = \"$target\"" fuzz/Cargo.toml
    test -s "fuzz/fuzz_targets/$target.rs"
    grep -F -q "$target" scripts/check_fuzz.sh
    grep -F -q "$target" scripts/check_fuzz_corpus.sh
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

echo "2.0 fuzz campaigns: complete adversarial target and property evidence ok"
