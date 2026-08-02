#!/usr/bin/env sh
set -eu

echo "checks: formatting"
cargo fmt --all --check

echo "checks: release metadata"
scripts/validate-release-metadata.sh

echo "checks: generated-evidence source provenance"
scripts/test-evidence-source.sh

echo "checks: portable CT assembly symbol matching"
scripts/test-ct-asm-symbols.sh

echo "checks: crate publish plan"
scripts/release_crates.py --check
python3 scripts/test-release-crates.py
scripts/test-release-readiness.sh

echo "checks: MSRV policy"
scripts/validate-msrv-policy.sh

echo "checks: documentation versions"
scripts/validate-doc-versions.sh

echo "checks: 2.0 governance"
scripts/validate-2.0-governance.sh

echo "checks: public API audit"
scripts/validate-api-audit.sh

echo "checks: 2.0 API migration ledger"
scripts/validate-2.0-api-ledger.sh

echo "checks: frozen 1.3.9 and 2.0 development public API snapshots"
scripts/check-api-snapshots.sh

echo "checks: 2.0 migration examples"
scripts/check-2.0-migration-smoke.sh

echo "checks: 2.0 feature contract"
scripts/check-2.0-feature-contract.sh

echo "checks: 2.0 crate skeleton"
scripts/validate-2.0-skeleton.sh

echo "checks: 2.0 validated alphabet"
scripts/check-2.0-alphabet.sh

echo "checks: 2.0 codec specifications"
scripts/check-2.0-specifications.sh

echo "checks: 2.0 line wrapping"
scripts/check-2.0-line-wrapping.sh

echo "checks: 2.0 operation contracts"
scripts/check-2.0-contracts.sh

echo "checks: 2.0 incremental encoder"
scripts/check-2.0-incremental-encoder.sh

echo "checks: 2.0 incremental padded decoder"
scripts/check-2.0-incremental-padded-decoder.sh

echo "checks: 2.0 incremental decoder finalization"
scripts/check-2.0-incremental-decoder-finalization.sh

echo "checks: 2.0 transactional one-shot operations"
scripts/check-2.0-one-shot.sh

echo "checks: 2.0 const transforms and bounded buffers"
scripts/check-2.0-const-buffers.sh

echo "checks: 2.0 in-place transforms"
scripts/check-2.0-in-place.sh

echo "checks: 2.0 formatting, append, and chunks"
scripts/check-2.0-format-append-chunks.sh

echo "checks: 2.0 WHATWG forgiving Base64"
scripts/check-2.0-web-forgiving.sh

echo "checks: 2.0 profiles and protocol terminology"
scripts/check-2.0-profiles.sh

echo "checks: 2.0 secret storage and explicit exposure"
scripts/check-2.0-secret-storage.sh

echo "checks: 2.0 bounded secret decoder"
scripts/check-2.0-secret-decoder.sh

echo "checks: RFC 4648 source lock"
scripts/verify-rfcs.sh
scripts/check-rfc-source-mutations.py

echo "checks: cross-crate semantic corpus"
scripts/check-semantic-corpus.sh

echo "checks: file line budget"
scripts/validate-file-line-budget.sh

echo "checks: minimal dependency graph"
scripts/validate-dependencies.sh

echo "checks: companion crates"
scripts/check_companion_crates.sh

echo "checks: reserved feature placeholders"
scripts/check_reserved_features.sh

echo "checks: unsafe boundary"
scripts/validate-unsafe-boundary.sh

echo "checks: wasm SIMD posture"
scripts/validate-wasm-posture.sh

echo "checks: big-endian QEMU posture"
scripts/validate-big-endian-posture.sh

echo "checks: RISC-V QEMU posture"
scripts/validate-riscv-posture.sh

echo "checks: AArch64 SVE QEMU posture"
scripts/validate-sve-posture.sh

echo "checks: wasm SIMD codegen evidence"
BASE64_NG_ALLOW_DIRTY_EVIDENCE=1 scripts/generate_wasm_simd_evidence.sh

echo "checks: wasm SIMD runtime dispatch"
scripts/check_wasm_runtime_dispatch.sh

echo "checks: 2.0 wasm loader package"
scripts/check-2.0-wasm-loader.sh

echo "checks: 2.0 wasm loader Chromium dispatch"
scripts/check_wasm_loader_browser_dispatch.sh

echo "checks: 2.0 wasm loader Firefox dispatch"
scripts/check_wasm_loader_browser_firefox_dispatch.sh

echo "checks: wasm SIMD browser dispatch"
scripts/check_wasm_browser_dispatch.sh

echo "checks: wasm SIMD Firefox dispatch"
scripts/check_wasm_browser_firefox_dispatch.sh

echo "checks: SIMD admission policy"
scripts/validate-simd-admission.sh

echo "checks: SIMD encode admission draft"
scripts/validate-simd-encode-admission-draft.sh

echo "checks: SIMD non-standard surface review"
scripts/validate-simd-non-standard-surfaces.sh

echo "checks: SIMD feature bundles"
scripts/check_simd_feature_bundles.sh

echo "checks: panic policy"
scripts/validate-panic-policy.sh

echo "checks: constant-time policy"
scripts/validate-constant-time-policy.sh

echo "checks: 2.0 secret encoder"
scripts/check-2.0-secret-encoder.sh

echo "checks: 2.0 secret capability split"
scripts/check-2.0-secret-capabilities.sh

echo "checks: 2.0 assurance and protected memory"
scripts/check-2.0-assurance.sh

echo "checks: 2.0 per-operation reporting"
scripts/check-2.0-operation-reporting.sh

echo "checks: 2.0 backend health and quarantine"
scripts/check-2.0-backend-health.sh

echo "checks: 2.0 x86 encode hot paths"
scripts/check-2.0-x86-encode-hot-paths.sh

echo "checks: 2.0 x86 decode hot paths"
scripts/check-2.0-x86-decode-hot-paths.sh

echo "checks: 2.0 AArch64 NEON hot paths"
scripts/check-2.0-neon-hot-paths.sh

echo "checks: 2.0 final dispatch and performance matrix"
scripts/validate-2.0-dispatch-matrix.sh

echo "checks: 2.0 synchronous I/O"
scripts/check-2.0-sync-io.sh

echo "checks: 2.0 bytes integration"
scripts/check-2.0-bytes.sh

echo "checks: dudect timing harness"
scripts/check_dudect.sh

echo "checks: fuzz harness"
scripts/check_fuzz.sh

echo "checks: performance harness"
scripts/check_perf.sh

echo "checks: clippy default"
cargo clippy --all-targets -- -D warnings

echo "checks: clippy all features"
cargo clippy --all-targets --all-features -- -D warnings

echo "checks: no_std library build"
cargo check --no-default-features --lib

echo "checks: no-alloc portability smoke"
scripts/check_no_alloc_smoke.sh

echo "checks: migration guide smoke"
scripts/check_migration_smoke.sh

echo "checks: tests default"
cargo test --all-targets

echo "checks: tests all features"
cargo test --all-targets --all-features

echo "checks: tests no default features"
cargo test --no-default-features --all-targets

echo "checks: doctests"
cargo test --doc --all-features

echo "checks: doctests no default features"
cargo test --doc --no-default-features

echo "checks: docs"
cargo doc --no-deps --all-features

echo "checks: docs no default features"
cargo doc --no-deps --no-default-features

echo "checks: dependency policy"
cargo deny check

echo "checks: RustSec advisories"
cargo audit

echo "checks: license inventory"
cargo license --json >/tmp/base64-ng-cargo-license.json
test -s /tmp/base64-ng-cargo-license.json

echo "checks: ok"
