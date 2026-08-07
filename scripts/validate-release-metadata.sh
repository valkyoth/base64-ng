#!/usr/bin/env sh
set -eu

package_name="$(
    sed -n 's/^name = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
cargo_version="$(
    sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
cargo_rust_version="$(
    sed -n 's/^rust-version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
toolchain_version="$(
    sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml | sed -n '1p'
)"
release_policy="$(
    sed -n 's/^policy = "\([^"]*\)"/\1/p' release-crates.toml | sed -n '1p'
)"

if [ "$package_name" != "base64-ng" ]; then
    echo "release metadata: package name must be base64-ng" >&2
    exit 1
fi

if [ -z "$cargo_version" ]; then
    echo "release metadata: Cargo.toml package version is missing" >&2
    exit 1
fi

if [ -z "$cargo_rust_version" ]; then
    echo "release metadata: Cargo.toml rust-version is missing" >&2
    exit 1
fi

if [ -z "$toolchain_version" ]; then
    echo "release metadata: rust-toolchain.toml channel is missing" >&2
    exit 1
fi

case "$toolchain_version" in
    *-*)
        echo "release metadata: rust-toolchain.toml must pin a stable release toolchain, got $toolchain_version" >&2
        exit 1
        ;;
esac

if ! grep -q '^license = "MIT OR Apache-2.0"$' Cargo.toml; then
    echo "release metadata: Cargo.toml must declare license = \"MIT OR Apache-2.0\"" >&2
    exit 1
fi

if ! grep -q '^repository = "https://github.com/valkyoth/base64-ng"$' Cargo.toml; then
    echo "release metadata: Cargo.toml repository must be https://github.com/valkyoth/base64-ng" >&2
    exit 1
fi

if ! grep -q '^homepage = "https://github.com/valkyoth/base64-ng"$' Cargo.toml; then
    echo "release metadata: Cargo.toml homepage must be https://github.com/valkyoth/base64-ng" >&2
    exit 1
fi

test -s LICENSE-MIT
test -s LICENSE-APACHE
test -s rust-toolchain.toml
test -s deny.toml
test -s release-crates.toml
test -s README.md
test -s CONTRIBUTING.md
test -s SECURITY.md
test -d release-notes
test -s release-notes/RELEASE_NOTES_2.0.0.md
test -s security/pentest/README.md
test -s docs/API_AUDIT.md
test -s docs/2.0_GOVERNANCE.md
test -s docs/2.0_API_MIGRATION_LEDGER.md
test -s docs/2.0_PACKAGE_TOPOLOGY.md
test -s docs/2.0_SECRET_STORAGE_AND_EXPOSURE.md
test -s docs/2.0_ASSURANCE_AND_PROTECTED_MEMORY.md
test -s docs/2.0_OPERATION_REPORTING.md
test -s docs/2.0_DISPATCH_AND_PERFORMANCE_MATRIX.md
test -s docs/2.0_DERIVE_HARDENING.md
test -s docs/2.0_PROTOCOL_REGISTRY.md
test -s docs/2.0_FORMAL_VERIFICATION.md
test -s docs/2.0_TIMING_AND_CODEGEN.md
test -s docs/2.0_RELEASE_FREEZE.md
test -s kani/harnesses.tsv
test -s protocol-registry/v1/SHA256SUMS
test -s protocol-registry/v1/protocols.tsv
test -s protocol-registry/v1/configurations.tsv
test -s protocol-registry/v1/cases.tsv
test -s protocol-registry/v1/provenance.tsv
test -s packages/base64-ng-wasm-loader/package.json
test -s packages/base64-ng-wasm-loader/package-lock.json
test -s packages/base64-ng-wasm-loader/README.md
test -s packages/base64-ng-wasm-loader/artifacts/base64-ng-scalar.wasm
test -s packages/base64-ng-wasm-loader/artifacts/base64-ng-simd128.wasm
grep -F -q '"name": "@valkyoth/base64-ng-wasm-loader"' \
    packages/base64-ng-wasm-loader/package.json
grep -F -q '"access": "public"' packages/base64-ng-wasm-loader/package.json
test -s api-snapshots/README.md
test -s docs/ASYNC.md
test -s docs/BENCHMARKS.md
test -s docs/BIG_ENDIAN_QEMU_REVIEW.md
test -s docs/CONSTANT_TIME.md
test -s docs/CT_ASM_REVIEW.md
test -s docs/DEPENDENCIES.md
test -s docs/DUDECT.md
test -s docs/FUZZING.md
test -s docs/INVARIANTS.md
test -s docs/KANI.md
test -s docs/MIGRATION.md
test -s docs/PANIC_POLICY.md
test -s docs/PLAN.md
test -s docs/RELEASE.md
test -s docs/RELEASE_EVIDENCE.md
test -s docs/SECURITY_CONTROLS.md
test -s docs/SIMD.md
test -s docs/SIMD_ADMISSION.md
test -s docs/SIMD_ENCODE_ADMISSION_DRAFT.md
test -s docs/TRUST.md
test -s docs/UNSAFE.md
test -s .github/workflows/security-audit.yml
test -x scripts/release_crates.py
test -x scripts/generate_release_history.py
test -x scripts/validate-release-readiness.sh
test -x scripts/finalize-release-evidence.sh
test -x scripts/sign-release-evidence.sh
test -x scripts/seal-release-evidence.sh
test -x scripts/verify-release-evidence-signature.sh
test -x scripts/verify-release-evidence-artifacts.py
test -x scripts/release_wasm_loader.sh
test -x scripts/verify-release-tag.sh
test -x scripts/validate-release-evidence-outcomes.sh
test -x scripts/test-release-evidence-outcomes.sh
test -x scripts/test-dudect-release-policy.sh
test -x scripts/test-release-tag-policy.sh
test -x scripts/test-release-evidence-signature.sh
test -x scripts/test-release-evidence-artifacts.py
test -s security/release-signers
test -s security/evidence-signers
test -s scripts/test-release-crates.py
test -x scripts/test-release-readiness.sh
test -s scripts/ct-asm-symbols.sh
test -x scripts/test-ct-asm-symbols.sh
test -x scripts/check-2.0-operation-reporting.sh
test -x scripts/validate-2.0-dispatch-matrix.sh
test -x scripts/capture-2.0-neon-admission.sh
test -x scripts/validate-neon-admission-bundle.py
test -x scripts/capture-2.0-riscv-admission.sh
test -x scripts/validate-rvv-admission-bundle.py
if ! awk '
    /name: Format, lint, test, and audit/ { in_checks = 1 }
    in_checks && /fetch-depth: 0/ { found = 1 }
    in_checks && /^  [A-Za-z0-9_-]+:/ && !/name: Format, lint, test, and audit/ { exit }
    END { exit(found ? 0 : 1) }
' .github/workflows/ci.yml; then
    echo "release metadata: main checks job must fetch full Git history for retained evidence provenance" >&2
    exit 1
fi
test -x scripts/test-neon-admission-bundle.py
test -x scripts/test-rvv-admission-bundle.py
test -x scripts/validate-2.0-checkpoint-record.py
test -x scripts/test-2.0-checkpoint-record.py

if [ "$(sed -n '1p' scripts/release_crates.py)" != "#!/usr/bin/env python3" ]; then
    echo "release metadata: scripts/release_crates.py must use #!/usr/bin/env python3" >&2
    exit 1
fi

if ! grep -F -q '[release]' release-crates.toml; then
    echo "release metadata: release-crates.toml is missing [release]" >&2
    exit 1
fi

if ! grep -F -q "version = \"$cargo_version\"" release-crates.toml; then
    echo "release metadata: release-crates.toml version must match Cargo.toml version $cargo_version" >&2
    exit 1
fi

if [ "$release_policy" = "development-blocked" ]; then
    if grep -F -q 'publish = true' release-crates.toml; then
        echo "release metadata: development plan selected a crate for publication" >&2
        exit 1
    fi
fi

if [ "$release_policy" = "synced-family" ]; then
    publish_count="$(grep -F -c 'publish = true' release-crates.toml)"
    if [ "$publish_count" -ne 13 ] || grep -F -q 'publish = false' release-crates.toml; then
        echo "release metadata: 2.0 synchronized family must publish all 13 Rust packages" >&2
        exit 1
    fi
fi

for required_script in \
    "scripts/check-2.0-feature-contract.sh" \
    "scripts/check-2.0-const-buffers.sh" \
    "scripts/check-2.0-format-append-chunks.sh" \
    "scripts/check-2.0-bytes.sh" \
    "scripts/check-2.0-tokio-writers.sh" \
    "scripts/check-2.0-serde.sh" \
    "scripts/check-2.0-subtle.sh" \
    "scripts/check-2.0-derive.sh" \
    "scripts/check-2.0-imap.sh" \
    "scripts/check-2.0-mime-body.sh" \
    "scripts/check-2.0-multibase.sh" \
    "scripts/check-2.0-password.sh" \
    "scripts/check-2.0-openpgp.sh" \
    "scripts/check-protocol-registry.sh" \
    "scripts/check_kani.sh" \
    "scripts/check_kani_advanced.sh" \
    "scripts/check-2.0-pem.sh" \
    "scripts/check-2.0-in-place.sh" \
    "scripts/check-2.0-in-place-sanitizers.sh" \
    "scripts/check-2.0-fuzz-campaigns.sh" \
    "scripts/check-2.0-wasm-loader.sh" \
    "scripts/check-2.0-release-freeze.sh" \
    "scripts/check-2.0-neon-hot-paths.sh" \
    "scripts/capture-2.0-neon-admission.sh" \
    "scripts/capture-2.0-riscv-admission.sh" \
    "scripts/validate-2.0-dispatch-matrix.sh" \
    "scripts/check-2.0-x86-decode-hot-paths.sh" \
    "scripts/check-2.0-profiles.sh" \
    "scripts/check-2.0-secret-capabilities.sh" \
    "scripts/check-2.0-assurance.sh" \
    "scripts/check-2.0-secret-storage.sh" \
    "scripts/check-2.0-web-forgiving.sh" \
    "scripts/check-2.0-migration-smoke.sh" \
    "scripts/check-api-snapshots.sh" \
    "scripts/validate-2.0-api-ledger.sh" \
    "scripts/check_backend_evidence.sh" \
    "scripts/check_big_endian_hardware.sh" \
    "scripts/check_big_endian_qemu.sh" \
    "scripts/check_big_endian_intrinsics_status.sh" \
    "scripts/cargo-deny-check.sh" \
    "scripts/check_riscv_hardware.sh" \
    "scripts/check_riscv_qemu.sh" \
    "scripts/check_riscv_intrinsics_status.sh" \
    "scripts/check_sve_hardware.sh" \
    "scripts/check_sve_qemu.sh" \
    "scripts/check_scheduled_advisories.sh" \
    "scripts/validate-api-audit.sh" \
    "scripts/validate-2.0-governance.sh" \
    "scripts/validate-big-endian-posture.sh" \
    "scripts/validate-big-endian-byte-order.sh" \
    "scripts/validate-riscv-posture.sh" \
    "scripts/check_dudect.sh" \
    "scripts/capture-fuzz-shard.sh" \
    "scripts/check-fuzz-shard-progress.sh" \
    "scripts/aggregate-fuzz-shards.sh" \
    "scripts/check_fuzz.sh" \
    "scripts/check_fuzz_corpus.sh" \
    "scripts/check_kani.sh" \
    "scripts/check_miri.sh" \
    "scripts/check_migration_smoke.sh" \
    "scripts/check_no_alloc_smoke.sh" \
    "scripts/check_perf.sh" \
    "scripts/check_reserved_features.sh" \
    "scripts/check_simd_feature_bundles.sh" \
    "scripts/check_targets.sh" \
    "scripts/check_high_assurance_policy.sh" \
    "scripts/check_wasm_wipe_policy.sh" \
    "scripts/check_wasm_loader_browser_dispatch.sh" \
    "scripts/check_wasm_loader_browser_firefox_dispatch.sh" \
    "scripts/check_wasm_loader_browser_safari_dispatch.sh" \
    "scripts/checks.sh" \
    "scripts/ci_install_rust.sh" \
    "scripts/generate-sbom.sh" \
    "scripts/generate_ct_asm_evidence.sh" \
    "scripts/generate_rvv_asm_evidence.sh" \
    "scripts/generate_sve_asm_evidence.sh" \
    "scripts/test-ct-asm-symbols.sh" \
    "scripts/test-evidence-source.sh" \
    "scripts/reproducible_build_check.sh" \
    "scripts/stable_release_gate.sh" \
    "scripts/finalize-release-evidence.sh" \
    "scripts/sign-release-evidence.sh" \
    "scripts/seal-release-evidence.sh" \
    "scripts/verify-release-evidence-signature.sh" \
    "scripts/test-release-evidence-signature.sh" \
    "scripts/release_wasm_loader.sh" \
    "scripts/verify-release-tag.sh" \
    "scripts/validate-release-evidence-outcomes.sh" \
    "scripts/test-release-evidence-outcomes.sh" \
    "scripts/test-dudect-release-policy.sh" \
    "scripts/test-release-tag-policy.sh" \
    "scripts/validate-constant-time-policy.sh" \
    "scripts/validate-2.0-timing-boundaries.sh" \
    "scripts/validate-dependencies.sh" \
    "scripts/validate-file-line-budget.sh" \
    "scripts/validate-doc-versions.sh" \
    "scripts/validate-msrv-policy.sh" \
    "scripts/validate-panic-policy.sh" \
    "scripts/validate-release-readiness.sh" \
    "scripts/validate-release-metadata.sh" \
    "scripts/validate-wasm-posture.sh" \
    "scripts/validate-simd-encode-admission-draft.sh" \
    "scripts/validate-simd-admission.sh" \
    "scripts/validate-unsafe-boundary.sh"
do
    if [ ! -x "$required_script" ]; then
        echo "release metadata: $required_script must be executable" >&2
        exit 1
    fi

    if [ "$(sed -n '1p' "$required_script")" != "#!/usr/bin/env sh" ]; then
        echo "release metadata: $required_script must use #!/usr/bin/env sh" >&2
        exit 1
    fi
done

for required_python_script in \
    "scripts/fuzz_shard_evidence.py" \
    "scripts/test-fuzz-shard-evidence.py" \
    "scripts/fuzz_evidence_session.py" \
    "scripts/fuzz_evidence_jobs.py" \
    "scripts/manage-fuzz-evidence.py" \
    "scripts/test-fuzz-evidence-manager.py" \
    "scripts/validate-neon-admission-bundle.py" \
    "scripts/test-neon-admission-bundle.py" \
    "scripts/validate-rvv-admission-bundle.py" \
    "scripts/test-rvv-admission-bundle.py" \
    "scripts/verify-release-evidence-artifacts.py" \
    "scripts/test-release-evidence-artifacts.py" \
    "scripts/validate-2.0-checkpoint-record.py" \
    "scripts/test-2.0-checkpoint-record.py"
do
    if [ ! -x "$required_python_script" ]; then
        echo "release metadata: $required_python_script must be executable" >&2
        exit 1
    fi
    if [ "$(sed -n '1p' "$required_python_script")" != "#!/usr/bin/env python3" ]; then
        echo "release metadata: $required_python_script must use #!/usr/bin/env python3" >&2
        exit 1
    fi
done

for shell_script in scripts/*.sh; do
    if grep -n -E '(^|[[:space:]])rg([[:space:]]|$)' "$shell_script"; then
        echo "release metadata: shell policy scripts must not require ripgrep: $shell_script" >&2
        exit 1
    fi
done

for evidence_generator in \
    scripts/generate_ct_asm_evidence.sh \
    scripts/generate_simd_asm_evidence.sh \
    scripts/generate_wasm_simd_evidence.sh
do
    for required_evidence_text in \
        'mktemp -d' \
        '. scripts/evidence-source.sh' \
        'evidence_capture_source' \
        'evidence_verify_source' \
        'CARGO_INCREMENTAL=0' \
        'cargo rustc --locked' \
        'expected exactly one fresh'
    do
        if ! grep -F -q -- "$required_evidence_text" "$evidence_generator"; then
            echo "release metadata: $evidence_generator is missing fresh evidence control: $required_evidence_text" >&2
            exit 1
        fi
    done
done

for required_ct_evidence_text in \
    'BASE64_NG_CT_ASM_TARGET' \
    'WIPE_PRIMITIVE_REVISION' \
    'runtime_wipe_generation=operation-report-specific' \
    'wipe_scope=logical-range volatile overwrite' \
    'require_reviewed_symbols' \
    'decode_symbol'
do
    if ! grep -F -q -- "$required_ct_evidence_text" scripts/generate_ct_asm_evidence.sh; then
        echo "release metadata: CT evidence boundary is missing: $required_ct_evidence_text" >&2
        exit 1
    fi
done

for required_dudect_evidence_text in \
    'evidence_capture_source "dudect timing evidence"' \
    'evidence_verify_source "dudect timing evidence"' \
    'evidence_write_source_manifest' \
    'public-length' \
    'post-gate release copy'
do
    if ! grep -F -q -- "$required_dudect_evidence_text" scripts/check_dudect.sh dudect/src/main.rs; then
        echo "release metadata: dudect evidence boundary is missing: $required_dudect_evidence_text" >&2
        exit 1
    fi
done

for required_provenance_text in \
    'evidence generation requires a Git worktree' \
    'refusing to generate release evidence from a dirty tree' \
    'source or lockfile changed during evidence generation' \
    'dirty-development-only' \
    'EVIDENCE_LOCK_RECORD="$(evidence_checksum_file Cargo.lock)"' \
    'evidence_require_exact_manifest_key' \
    'evidence_require_singleton_manifest_line' \
    'evidence_require_clean_source_manifest'
do
    if ! grep -F -q -- "$required_provenance_text" scripts/evidence-source.sh; then
        echo "release metadata: evidence source boundary is missing: $required_provenance_text" >&2
        exit 1
    fi
done

for required_finalizer_text in \
    'require_source_manifest_for' \
    'campaign_commit="${BASE64_NG_REUSE_EVIDENCE_FROM:-$EVIDENCE_SOURCE_COMMIT}"' \
    'evidence tree contains a symbolic link' \
    'evidence_mode=metadata-equivalent' \
    'manifest remains unsigned until the isolated sealing step'
do
    if ! grep -F -q -- "$required_finalizer_text" scripts/finalize-release-evidence.sh; then
        echo "release metadata: final evidence provenance validation is missing: $required_finalizer_text" >&2
        exit 1
    fi
done

if ! grep -F -q 'scripts/verify-release-evidence-artifacts.py' \
    scripts/validate-release-readiness.sh; then
    echo "release metadata: readiness does not verify signed artifact contents" >&2
    exit 1
fi

for required_sealing_text in \
    'scripts/sign-release-evidence.sh "$manifest"' \
    'unset BASE64_NG_EVIDENCE_SIGNING_KEY' \
    'scripts/validate-release-readiness.sh "$tag"'
do
    if ! grep -F -q -- "$required_sealing_text" scripts/seal-release-evidence.sh; then
        echo "release metadata: isolated evidence sealing is missing: $required_sealing_text" >&2
        exit 1
    fi
done

for required_equivalence_text in \
    'merge-base' \
    'non-metadata paths changed' \
    'protected repository contents differ' \
    'retained FINAL-MANIFEST is not bound to the clean evidence commit' \
    'package_evidence=must-be-regenerated' \
    'target/release-evidence/big-endian-qemu/' \
    'target/release-evidence/riscv-qemu/' \
    'target/release-evidence/sve-qemu/' \
    'retained FINAL-MANIFEST signature is invalid'
do
    if ! grep -F -q -- "$required_equivalence_text" scripts/evidence-equivalence.py; then
        echo "release metadata: evidence equivalence gate is missing: $required_equivalence_text" >&2
        exit 1
    fi
done

for required_release_provenance_text in \
    'unset BASE64_NG_ALLOW_DIRTY_EVIDENCE' \
    'evidence_capture_source "stable release gate"' \
    'scripts/generate_ct_asm_evidence.sh' \
    'scripts/generate_simd_asm_evidence.sh' \
    'scripts/generate_wasm_simd_evidence.sh' \
    'BASE64_NG_REUSE_EVIDENCE_FROM' \
    'refuse to expose the evidence signing key to build/test subprocesses' \
    'scripts/evidence-equivalence.py' \
    'evidence_verify_source "stable release gate"'
do
    if ! grep -F -q -- "$required_release_provenance_text" scripts/stable_release_gate.sh; then
        echo "release metadata: stable release provenance gate is missing: $required_release_provenance_text" >&2
        exit 1
    fi
done

for required_scheduled_audit_text in \
    'cron: "17 3 * * *"' \
    "workflow_dispatch:" \
    "scripts/check_scheduled_advisories.sh"
do
    if ! grep -F -q "$required_scheduled_audit_text" .github/workflows/security-audit.yml; then
        echo "release metadata: scheduled security audit is missing $required_scheduled_audit_text" >&2
        exit 1
    fi
done

for required_public_api_text in \
    'public_api_toolchain="nightly-2026-07-13"' \
    'rustup toolchain install nightly-2026-07-13 --profile minimal' \
    'rustup which --toolchain "$public_api_toolchain" cargo' \
    'PATH="$public_api_path:$PATH"'
do
    if ! grep -F -q "$required_public_api_text" \
        scripts/check-api-snapshots.sh .github/workflows/ci.yml
    then
        echo "release metadata: public API toolchain pin is missing $required_public_api_text" >&2
        exit 1
    fi
done

for required_ci_evidence_text in \
    'rustup target add aarch64-unknown-linux-gnu wasm32-unknown-unknown' \
    'binutils-aarch64-linux-gnu'
do
    if ! grep -F -q "$required_ci_evidence_text" .github/workflows/ci.yml; then
        echo "release metadata: CI evidence setup is missing $required_ci_evidence_text" >&2
        exit 1
    fi
done

if ! awk '
    /rustup target add aarch64-unknown-linux-gnu wasm32-unknown-unknown/ {
        target_install = NR
    }
    /run: scripts\/checks\.sh/ {
        checks = NR
    }
    END {
        exit !(target_install > 0 && checks > 0 && target_install < checks)
    }
' .github/workflows/ci.yml; then
    echo "release metadata: complete-check targets must be installed before scripts/checks.sh" >&2
    exit 1
fi

qemu_serial_count="$(grep -F -c -- '--test-threads=1' scripts/check_big_endian_qemu.sh || true)"
if [ "$qemu_serial_count" -ne 5 ]; then
    echo "release metadata: all five QEMU guest test commands must retain serial libtest execution" >&2
    exit 1
fi

for required_sve_binutils_text in \
    'for candidate_prefix in aarch64-linux-gnu aarch64-suse-linux' \
    'objdump="${binutils_prefix}objdump"' \
    'nm="${binutils_prefix}nm"' \
    'readelf="${binutils_prefix}readelf"'
do
    if ! grep -F -q "$required_sve_binutils_text" scripts/generate_sve_asm_evidence.sh; then
        echo "release metadata: SVE evidence must require target-aware $required_sve_binutils_text" >&2
        exit 1
    fi
done

if [ "$(sed -n '1p' scripts/validate-release-readiness.sh)" != "#!/usr/bin/env sh" ]; then
    echo "release metadata: scripts/validate-release-readiness.sh must use #!/usr/bin/env sh" >&2
    exit 1
fi

if [ "$(sed -n '1p' scripts/test-release-readiness.sh)" != "#!/usr/bin/env sh" ]; then
    echo "release metadata: scripts/test-release-readiness.sh must use #!/usr/bin/env sh" >&2
    exit 1
fi

if [ "$(sed -n '1p' scripts/generate_release_history.py)" != "#!/usr/bin/env python3" ]; then
    echo "release metadata: scripts/generate_release_history.py must use #!/usr/bin/env python3" >&2
    exit 1
fi

if ! grep -q '^The MIT License (MIT)$' LICENSE-MIT; then
    echo "release metadata: LICENSE-MIT does not look like the canonical MIT license" >&2
    exit 1
fi

if ! grep -q 'Apache License' LICENSE-APACHE || ! grep -q 'Version 2.0, January 2004' LICENSE-APACHE; then
    echo "release metadata: LICENSE-APACHE does not look like the canonical Apache 2.0 license" >&2
    exit 1
fi

if ! grep -q "^## $cargo_version " CHANGELOG.md; then
    echo "release metadata: CHANGELOG.md is missing a section for Cargo version $cargo_version" >&2
    exit 1
fi

for required_release_doc_text in \
    "native byte-array and \`FromStr\` interop surfaces" \
    "Linux, FreeBSD, wasm32, ARM, and Cortex-M targets" \
    "BASE64_NG_RUN_FUZZ_SMOKE=1 scripts/check_fuzz.sh"
do
    if ! grep -q "$required_release_doc_text" docs/RELEASE_EVIDENCE.md docs/RELEASE.md; then
        echo "release metadata: release docs are missing required text: $required_release_doc_text" >&2
        exit 1
    fi
done

for required_release_gate_command in \
    "scripts/checks.sh" \
    "cargo nextest run --all-features" \
    "scripts/check_miri.sh" \
    "scripts/check-2.0-in-place-sanitizers.sh" \
    "BASE64_NG_RUN_FUZZ_RELEASE=1" \
    "scripts/check_fuzz.sh" \
    "BASE64_NG_RUN_DUDECT=1" \
    "scripts/check_dudect.sh" \
    "scripts/check_targets.sh" \
    "scripts/check_big_endian_qemu.sh --all" \
    "scripts/check_riscv_qemu.sh" \
    "scripts/generate_rvv_asm_evidence.sh" \
    "scripts/check_sve_qemu.sh" \
    "scripts/generate_sve_asm_evidence.sh" \
    "scripts/check_no_alloc_smoke.sh" \
    "scripts/check_migration_smoke.sh" \
    "scripts/check_simd_feature_bundles.sh" \
    "scripts/check_backend_evidence.sh" \
    "scripts/check_kani.sh" \
    "scripts/check_kani_advanced.sh" \
    "scripts/validate-2.0-timing-boundaries.sh" \
    "scripts/generate_ct_asm_evidence.sh" \
    "scripts/generate-sbom.sh" \
    "scripts/reproducible_build_check.sh" \
    "scripts/finalize-release-evidence.sh" \
    'scripts/validate-release-readiness.sh "v${cargo_version}"'
do
    if ! grep -F -q "$required_release_gate_command" scripts/stable_release_gate.sh; then
        echo "release metadata: stable release gate is missing $required_release_gate_command" >&2
        exit 1
    fi
done

if ! grep -F -q '["scripts/stable_release_gate.sh", "candidate"]' scripts/release_crates.py; then
    echo "release metadata: post-tag full gate must use strict candidate mode" >&2
    exit 1
fi

for required_strict_gate_text in \
    'BASE64_NG_REQUIRE_MIRI=1' \
    'BASE64_NG_REQUIRE_SANITIZERS=1' \
    'BASE64_NG_FUZZ_SECONDS_PER_TARGET=3600' \
    'BASE64_NG_DUDECT_RELEASE=1' \
    'BASE64_NG_REQUIRE_KANI=1' \
    'BASE64_NG_KANI_ALL_ADVANCED=1' \
    'BASE64_NG_REQUIRE_COMMIT53_NATIVE=1' \
    'BASE64_NG_REQUIRE_RVV_NATIVE=1' \
    'scripts/finalize-release-evidence.sh'
do
    if ! grep -F -q "$required_strict_gate_text" scripts/stable_release_gate.sh; then
        echo "release metadata: strict release gate is missing $required_strict_gate_text" >&2
        exit 1
    fi
done

for required_rvv_release_text in \
    'BASE64_NG_RVV_ADMISSION_BUNDLE' \
    'BASE64_NG_EXPECTED_RVV_SOURCE_COMMIT' \
    'scripts/validate-rvv-admission-bundle.py "$rvv_bundle"' \
    'rvv=exact-linux-spacemit-x60-native-admission'
do
    if ! grep -F -q "$required_rvv_release_text" \
        scripts/check-2.0-memory-hardware-evidence.sh \
        scripts/validate-release-evidence-outcomes.sh
    then
        echo "release metadata: native RVV release gate is missing $required_rvv_release_text" >&2
        exit 1
    fi
done
if ! grep -F -q 'scripts/validate-rvv-admission-bundle.py "$rvv_native"' \
    scripts/finalize-release-evidence.sh; then
    echo "release metadata: final evidence index does not validate native RVV evidence" >&2
    exit 1
fi
if ! grep -F -q 'source_commit "$campaign_commit"' \
    scripts/finalize-release-evidence.sh; then
    echo "release metadata: reused native RVV evidence is not bound to the campaign commit" >&2
    exit 1
fi
if ! grep -F -q 'BASE64_NG_EXPECTED_RVV_SOURCE_COMMIT="$reuse_evidence_from"' \
    scripts/stable_release_gate.sh; then
    echo "release metadata: metadata-equivalent RVV evidence is not bound to the campaign" >&2
    exit 1
fi
if ! grep -F -q 'entry.is_symlink() or not entry.is_file()' \
    scripts/validate-rvv-admission-bundle.py; then
    echo "release metadata: native RVV validator does not reject symbolic links" >&2
    exit 1
fi
if ! grep -F -q 'target/release-evidence/riscv-native-admission/' \
    scripts/evidence-equivalence.py; then
    echo "release metadata: retained evidence policy omits native RVV artifacts" >&2
    exit 1
fi

for required_wasm_publish_text in \
    'scripts/check-2.0-wasm-loader.sh' \
    'git status --porcelain --untracked-files=all' \
    'scripts/verify-release-tag.sh "$tag"' \
    'npm publish --dry-run' \
    'npm publish --provenance'
do
    if ! grep -F -q "$required_wasm_publish_text" scripts/release_wasm_loader.sh; then
        echo "release metadata: wasm publisher is missing $required_wasm_publish_text" >&2
        exit 1
    fi
done

for required_trust_text in \
	"Runtime dependencies | Zero external crates" \
	"Per-operation backend | \`encode_backend\` and \`strict_decode_backend\` independently report admitted ordinary dispatch" \
	"\`secret_decode_backend\` is always \`scalar-constant-time-oriented\`" \
	"no formal cryptographic constant-time guarantee" \
	"formally verified cryptographic constant-time behavior" \
	"Wrapped and legacy decode may enter the admitted strict decode backend only after scalar line-profile validation" \
	"Strict in-place encode and decode may enter admitted backends only after stack staging" \
	"custom-alphabet, CT secret, or broader wasm/browser" \
	"automatic \`no_std\` runtime probing or unattested \`no_std\` acceleration" \
	"async/Tokio support" \
    "serde or bytes integration"
do
    if ! grep -q "$required_trust_text" docs/TRUST.md; then
        echo "release metadata: trust dashboard is missing required text: $required_trust_text" >&2
        exit 1
    fi
done

for required_invariant_text in \
    "Chunk Reads" \
    "Output Writes" \
    "In-Place Decode" \
    "Constant-Time-Oriented Decode"
do
    if ! grep -F -q "$required_invariant_text" docs/INVARIANTS.md; then
        echo "release metadata: invariants doc is missing required text: $required_invariant_text" >&2
        exit 1
    fi
done

for required_ct_review_text in \
    "No formally verified cryptographic constant-time guarantee is claimed" \
    "Review Questions" \
    "ct_decode_alphabet_byte" \
    "Reviewer Notes"
do
    if ! grep -F -q "$required_ct_review_text" docs/CT_ASM_REVIEW.md; then
        echo "release metadata: ct asm review doc is missing required text: $required_ct_review_text" >&2
        exit 1
    fi
done

for required_kani_text in \
    "Kani runs are compiler-integration-sensitive" \
    "A Kani skip is not the same as a proof" \
    "Do not lower \`rust-version\` only to make Kani run" \
    "do not claim Kani-complete or formally verified behavior in the \`1.0.0\`" \
    "The stable \`1.0.0\` guarantee is the documented"
do
    if ! grep -F -q "$required_kani_text" docs/KANI.md; then
        echo "release metadata: Kani policy is missing required text: $required_kani_text" >&2
        exit 1
    fi
done

for required_dependency_review_text in \
    "v1.0 Final Admission Review" \
    "Optional ecosystem integrations may be admitted only as separate companion" \
    "base64-ng-sanitization\` is admitted as a companion crate" \
    "base64-ng-derive\` is admitted as a companion crate" \
    "base64-ng-serde\` is admitted as a companion crate" \
    "base64-ng-bytes\` is admitted as a companion crate" \
    "base64-ng-subtle\` is admitted as a companion crate" \
    "base64-ng-tokio\` is admitted as a companion crate" \
    "base64-ng-imap\` is admitted as a companion" \
    "base64-ng-mime\` is admitted as a companion" \
    "base64-ng-multibase\` is admitted as a companion" \
    "base64-ng-password\` is admitted as a companion" \
    "base64-ng-openpgp\` is admitted as a companion" \
    "base64-ng-pem\` is admitted as a companion" \
    "\`subtle\` is admitted only through \`base64-ng-subtle\`"
do
    if ! grep -F -q "$required_dependency_review_text" docs/DEPENDENCIES.md; then
        echo "release metadata: dependency policy is missing required review text: $required_dependency_review_text" >&2
        exit 1
    fi
done

case "$cargo_version" in
    *-*)
        required_readme_simd_status="Runtime-dispatched std \`x86\`/\`x86_64\` AVX-512 VBMI fixed-block encode"
        ;;
    *)
        required_readme_simd_status="Scalar by default; std x86/x86_64 encode selects SSSE3/SSE4.1, AVX2, or AVX-512 VBMI by length, strict decode selects SSSE3/SSE4.1 or AVX2"
        ;;
esac

for required_readme_text in \
    "Zero external runtime or development dependencies in \`Cargo.toml\`." \
    "$required_readme_simd_status" \
    "currently inert and dependency-free" \
    "no formal cryptographic guarantee" \
    "SBOM, and reproducible build check"
do
    if ! grep -q "$required_readme_text" README.md; then
        echo "release metadata: README.md is missing required text: $required_readme_text" >&2
        exit 1
    fi
done

for required_checks_command in \
    "scripts/validate-api-audit.sh" \
    "scripts/validate-big-endian-posture.sh" \
    "scripts/validate-riscv-posture.sh" \
    "scripts/validate-sve-posture.sh" \
    "scripts/validate-msrv-policy.sh" \
    "scripts/validate-wasm-posture.sh" \
    "scripts/validate-simd-encode-admission-draft.sh" \
    "scripts/release_crates.py --check" \
    "python3 scripts/test-release-crates.py" \
    "scripts/check_migration_smoke.sh" \
    "scripts/check-2.0-fuzz-campaigns.sh" \
    "cargo test --doc --all-features" \
    "cargo test --doc --no-default-features" \
    "cargo doc --no-deps --all-features" \
    "cargo doc --no-deps --no-default-features"
do
    if ! grep -F -q "$required_checks_command" scripts/checks.sh; then
        echo "release metadata: standard checks are missing $required_checks_command" >&2
        exit 1
    fi
done

for required_fuzz_gate_text in \
    "cargo audit --file fuzz/Cargo.lock" \
    "scripts/cargo-deny-check.sh fuzz/Cargo.toml fuzz/deny.toml" \
    "BASE64_NG_RUN_FUZZ_RELEASE=1" \
    "corpus-hashes:" \
    "minimization:"
do
    if ! grep -F -q "$required_fuzz_gate_text" scripts/check_fuzz.sh docs/FUZZING.md docs/RELEASE_EVIDENCE.md; then
        echo "release metadata: fuzz dependency gates are missing required text: $required_fuzz_gate_text" >&2
        exit 1
    fi
done

package_list="$(
    cargo package --locked --allow-dirty --list
)"

for required_package_file in \
    "CHANGELOG.md" \
    "CONTRIBUTING.md" \
    "deny.toml" \
    "LICENSE-APACHE" \
    "LICENSE-MIT" \
    "release-crates.toml" \
    "README.md" \
    "rust-toolchain.toml" \
    "SECURITY.md" \
    "api-snapshots/README.md" \
    "api-snapshots/v2.0.0/base64-ng.txt" \
    "api-snapshots/v2.0.0/base64-ng-bytes.txt" \
    "api-snapshots/v2.0.0/base64-ng-derive.txt" \
    "api-snapshots/v2.0.0/base64-ng-imap.txt" \
    "api-snapshots/v2.0.0/base64-ng-mime.txt" \
    "api-snapshots/v2.0.0/base64-ng-multibase.txt" \
    "api-snapshots/v2.0.0/base64-ng-password.txt" \
    "api-snapshots/v2.0.0/base64-ng-openpgp.txt" \
    "api-snapshots/v2.0.0/base64-ng-pem.txt" \
    "api-snapshots/v2.0.0/base64-ng-sanitization.txt" \
    "api-snapshots/v2.0.0/base64-ng-serde.txt" \
    "api-snapshots/v2.0.0/base64-ng-subtle.txt" \
    "api-snapshots/v2.0.0/base64-ng-tokio.txt" \
    "api-snapshots/v1.3.9/base64-ng.txt" \
    "api-snapshots/v1.3.9/base64-ng-bytes.txt" \
    "api-snapshots/v1.3.9/base64-ng-derive.txt" \
    "api-snapshots/v1.3.9/base64-ng-sanitization.txt" \
    "api-snapshots/v1.3.9/base64-ng-serde.txt" \
    "api-snapshots/v1.3.9/base64-ng-subtle.txt" \
    "api-snapshots/v1.3.9/base64-ng-tokio.txt" \
    "docs/2.0_API_MIGRATION_LEDGER.md" \
    "docs/2.0_BIG_ENDIAN_AUDIT.md" \
    "docs/2.0_CONST_AND_BOUNDED_BUFFERS.md" \
    "docs/2.0_FORMAT_APPEND_CHUNKS.md" \
    "docs/2.0_BYTES_INTEGRATION.md" \
    "docs/2.0_TOKIO_ASYNC_READ.md" \
    "docs/2.0_TOKIO_ASYNC_WRITE.md" \
    "docs/2.0_SERDE_INTEGRATION.md" \
    "docs/2.0_SUBTLE_EQUALITY.md" \
    "docs/2.0_DERIVE_HARDENING.md" \
    "docs/2.0_IMAP.md" \
    "docs/2.0_MIME_BODY.md" \
    "docs/2.0_MULTIBASE.md" \
    "docs/2.0_PASSWORD_RECORDS.md" \
    "docs/2.0_PROTOCOL_REGISTRY.md" \
    "docs/2.0_FORMAL_VERIFICATION.md" \
    "docs/2.0_TIMING_AND_CODEGEN.md" \
    "kani/README.md" \
    "kani/harnesses.tsv" \
    "docs/2.0_PEM.md" \
    "docs/2.0_IN_PLACE_OPERATIONS.md" \
    "docs/2.0_DEVICE_VERIFICATION_QUEUE.md" \
    "docs/2.0_PROFILES_AND_TERMINOLOGY.md" \
    "docs/2.0_SECRET_STORAGE_AND_EXPOSURE.md" \
    "docs/2.0_TRANSACTIONAL_ONE_SHOT.md" \
    "docs/2.0_PACKAGE_TOPOLOGY.md" \
    "docs/API_AUDIT.md" \
    "docs/ASYNC.md" \
    "docs/BENCHMARKS.md" \
    "docs/BIG_ENDIAN_QEMU_REVIEW.md" \
    "hardware-evidence/big-endian/README.md" \
    "hardware-evidence/big-endian/schema-v1.json" \
    "hardware-evidence/riscv/README.md" \
    "hardware-evidence/riscv/schema-v1.json" \
    "docs/RISCV_QEMU_REVIEW.md" \
    "docs/SVE_QEMU_REVIEW.md" \
    "hardware-evidence/sve/README.md" \
    "hardware-evidence/sve/schema-v1.json" \
    "docs/CONSTANT_TIME.md" \
    "docs/CT_ASM_REVIEW.md" \
    "docs/DEPENDENCIES.md" \
    "docs/DUDECT.md" \
    "docs/FUZZING.md" \
    "docs/INVARIANTS.md" \
    "docs/KANI.md" \
    "docs/MIGRATION.md" \
    "docs/NEON_COMMIT29_EVIDENCE.md" \
    "docs/PANIC_POLICY.md" \
    "docs/PLAN.md" \
    "docs/RELEASE.md" \
    "docs/RELEASE_EVIDENCE.md" \
    "docs/SECURITY_CONTROLS.md" \
    "docs/SIMD_ADMISSION.md" \
    "docs/SIMD_ENCODE_ADMISSION_DRAFT.md" \
    "docs/SIMD.md" \
    "docs/TRUST.md" \
    "docs/UNSAFE.md" \
    "portability/no_alloc_smoke/src/lib.rs" \
    "portability/migration_smoke/src/lib.rs" \
    "portability/2_0_migration_smoke/src/lib.rs" \
    "portability/2_0_no_std_alloc_smoke/src/lib.rs" \
    "portability/feature_contract_smoke/src/main.rs" \
    "portability/feature_unification_smoke/src/lib.rs" \
    "portability/aarch64_static_neon_smoke/src/main.rs" \
    "portability/x86_static_encode_smoke/src/main.rs" \
    "scripts/check-2.0-feature-contract.sh" \
    "scripts/check-2.0-const-buffers.sh" \
    "scripts/check-2.0-format-append-chunks.sh" \
    "scripts/check-2.0-bytes.sh" \
    "scripts/check-2.0-tokio-readers.sh" \
    "scripts/check-2.0-tokio-writers.sh" \
    "scripts/check-2.0-serde.sh" \
    "scripts/check-2.0-subtle.sh" \
    "scripts/check-2.0-derive.sh" \
    "scripts/check-2.0-imap.sh" \
    "scripts/check-2.0-mime-body.sh" \
    "scripts/check-2.0-multibase.sh" \
    "scripts/check-2.0-password.sh" \
    "scripts/check-2.0-openpgp.sh" \
    "scripts/check-protocol-registry.sh" \
    "scripts/validate-protocol-registry.py" \
    "scripts/check-protocol-registry-mutations.py" \
    "scripts/validate-password-spec.py" \
    "scripts/validate-kani-proof-inventory.py" \
    "scripts/check-2.0-pem.sh" \
    "scripts/check-2.0-in-place.sh" \
    "scripts/check-2.0-in-place-sanitizers.sh" \
    "scripts/check-2.0-fuzz-campaigns.sh" \
    "scripts/check-2.0-release-freeze.sh" \
    "scripts/check-2.0-neon-hot-paths.sh" \
    "scripts/capture-2.0-neon-admission.sh" \
    "scripts/validate-neon-admission-bundle.py" \
    "scripts/test-neon-admission-bundle.py" \
    "scripts/capture-2.0-riscv-admission.sh" \
    "scripts/validate-rvv-admission-bundle.py" \
    "scripts/test-rvv-admission-bundle.py" \
    "scripts/validate-2.0-checkpoint-record.py" \
    "scripts/test-2.0-checkpoint-record.py" \
    "scripts/check-2.0-one-shot.sh" \
    "scripts/check-2.0-profiles.sh" \
    "scripts/check-2.0-secret-capabilities.sh" \
    "scripts/check-2.0-secret-storage.sh" \
    "scripts/check_high_assurance_policy.sh" \
    "scripts/check-2.0-migration-smoke.sh" \
    "scripts/check-api-snapshots.sh" \
    "scripts/validate-2.0-api-ledger.sh" \
    "scripts/check_backend_evidence.sh" \
    "scripts/check_big_endian_hardware.sh" \
    "scripts/check_big_endian_qemu.sh" \
    "scripts/check_big_endian_intrinsics_status.sh" \
    "scripts/check_riscv_hardware.sh" \
    "scripts/check_riscv_qemu.sh" \
    "scripts/check_riscv_intrinsics_status.sh" \
    "scripts/check_sve_hardware.sh" \
    "scripts/check_sve_qemu.sh" \
    "scripts/ct-asm-symbols.sh" \
    "scripts/validate-api-audit.sh" \
    "scripts/validate-big-endian-posture.sh" \
    "scripts/validate-big-endian-byte-order.sh" \
    "scripts/validate-big-endian-hardware-evidence.py" \
    "scripts/test-big-endian-hardware-evidence.py" \
    "scripts/validate-riscv-posture.sh" \
    "scripts/validate-riscv-hardware-evidence.py" \
    "scripts/test-riscv-hardware-evidence.py" \
    "scripts/validate-sve-posture.sh" \
    "scripts/validate-sve-hardware-evidence.py" \
    "scripts/test-sve-hardware-evidence.py" \
    "scripts/check_dudect.sh" \
    "scripts/capture-fuzz-shard.sh" \
    "scripts/check-fuzz-shard-progress.sh" \
    "scripts/aggregate-fuzz-shards.sh" \
    "scripts/fuzz_shard_evidence.py" \
    "scripts/test-fuzz-shard-evidence.py" \
    "scripts/fuzz-release-targets.txt" \
    "scripts/fuzz-cargo-version.txt" \
    "scripts/fuzz_evidence_session.py" \
    "scripts/fuzz_evidence_jobs.py" \
    "scripts/manage-fuzz-evidence.py" \
    "scripts/test-fuzz-evidence-manager.py" \
    "scripts/check_fuzz.sh" \
    "scripts/check_fuzz_corpus.sh" \
    "scripts/check_kani.sh" \
    "scripts/check_miri.sh" \
    "scripts/check_migration_smoke.sh" \
    "scripts/check_no_alloc_smoke.sh" \
    "scripts/check_perf.sh" \
    "scripts/check_reserved_features.sh" \
    "scripts/check_simd_feature_bundles.sh" \
    "scripts/check_targets.sh" \
    "scripts/checks.sh" \
    "scripts/ci_install_rust.sh" \
    "scripts/generate-sbom.sh" \
    "scripts/generate_ct_asm_evidence.sh" \
    "scripts/generate_subtle_asm_evidence.sh" \
    "scripts/generate_rvv_asm_evidence.sh" \
    "scripts/generate_sve_asm_evidence.sh" \
    "scripts/generate_neon_asm_evidence.sh" \
    "scripts/test-ct-asm-symbols.sh" \
    "scripts/test-neon-performance.py" \
    "scripts/test-rvv-admission-bundle.py" \
    "scripts/reproducible_build_check.sh" \
    "scripts/release_crates.py" \
    "scripts/stable_release_gate.sh" \
    "scripts/test-release-crates.py" \
    "scripts/validate-constant-time-policy.sh" \
    "scripts/validate-2.0-timing-boundaries.sh" \
    "scripts/validate-dependencies.sh" \
    "scripts/validate-doc-versions.sh" \
    "scripts/validate-msrv-policy.sh" \
    "scripts/validate-neon-performance.py" \
    "scripts/validate-rvv-performance.py" \
    "scripts/validate-panic-policy.sh" \
    "scripts/validate-release-metadata.sh" \
    "scripts/validate-simd-encode-admission-draft.sh" \
    "scripts/validate-simd-admission.sh" \
    "scripts/validate-unsafe-boundary.sh" \
    "src/alphabet.rs" \
    "src/buffers/mod.rs" \
    "src/buffers/decoded.rs" \
    "src/buffers/encoded.rs" \
    "src/buffers/secret.rs" \
    "src/buffers/secret_conversions.rs" \
    "src/cleanup.rs" \
    "src/ct/mod.rs" \
    "src/ct/decode.rs" \
    "src/ct/equality.rs" \
    "src/ct/padded.rs" \
    "src/ct/unpadded.rs" \
    "src/engine/mod.rs" \
    "src/engine/decode.rs" \
    "src/engine/decode_in_place.rs" \
    "src/engine/encode.rs" \
    "src/engine/encode_in_place.rs" \
    "src/engine/stream.rs" \
    "src/engine/validate.rs" \
    "src/encode_surface_tests.rs" \
    "src/errors.rs" \
    "src/kani_proofs.rs" \
    "src/length.rs" \
    "src/lib.rs" \
    "src/profiles.rs" \
    "src/runtime/mod.rs" \
    "src/runtime/report.rs" \
    "src/scalar.rs" \
    "src/simd/mod.rs" \
    "src/simd/sve.rs" \
    "src/simd/tests.rs" \
    "src/stream/mod.rs" \
    "src/stream/common.rs" \
    "src/stream/decoder.rs" \
    "src/stream/decoder_reader.rs" \
    "src/stream/encoder.rs" \
    "src/stream/encoder_reader.rs" \
    "src/stream/queue.rs" \
    "src/tests.rs" \
    "src/wrap.rs" \
    "tests/rfc4648.rs"
do
    if ! printf '%s\n' "$package_list" | grep -qx "$required_package_file"; then
        echo "release metadata: package is missing $required_package_file" >&2
        exit 1
    fi
done

if printf '%s\n' "$package_list" | grep -q '^fuzz/'; then
    echo "release metadata: fuzz-only harness files must not be included in the published crate" >&2
    exit 1
fi

if printf '%s\n' "$package_list" | grep -q '^perf/'; then
    echo "release metadata: performance harness files must not be included in the published crate" >&2
    exit 1
fi

if printf '%s\n' "$package_list" | grep -q '^dudect/'; then
    echo "release metadata: dudect harness files must not be included in the published crate" >&2
    exit 1
fi

for required_no_alloc_symbol in \
    "clear_tail_surfaces" \
    "named_profile_surfaces" \
    "ct_stack_decode" \
    "custom_profile_surfaces" \
    "validate_only_surfaces" \
    "in_place_surfaces" \
    "native_interop_surfaces" \
    "BCRYPT" \
    "CRYPT" \
    "MIME" \
    "PEM"
do
    if ! grep -q "$required_no_alloc_symbol" portability/no_alloc_smoke/src/lib.rs; then
        echo "release metadata: no-alloc smoke is missing $required_no_alloc_symbol coverage" >&2
        exit 1
    fi
done

echo "release metadata: ok"
