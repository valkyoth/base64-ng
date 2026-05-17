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

if [ "$toolchain_version" != "$cargo_rust_version.0" ]; then
    echo "release metadata: rust-toolchain.toml channel $toolchain_version does not match Cargo.toml rust-version $cargo_rust_version" >&2
    exit 1
fi

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
test -s README.md
test -s CONTRIBUTING.md
test -s SECURITY.md
test -s docs/API_AUDIT.md
test -s docs/ASYNC.md
test -s docs/BENCHMARKS.md
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
test -s docs/TRUST.md
test -s docs/UNSAFE.md

for required_script in \
    "scripts/check_backend_evidence.sh" \
    "scripts/validate-api-audit.sh" \
    "scripts/check_dudect.sh" \
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
    "scripts/reproducible_build_check.sh" \
    "scripts/stable_release_gate.sh" \
    "scripts/validate-constant-time-policy.sh" \
    "scripts/validate-dependencies.sh" \
    "scripts/validate-doc-versions.sh" \
    "scripts/validate-msrv-policy.sh" \
    "scripts/validate-panic-policy.sh" \
    "scripts/validate-release-metadata.sh" \
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
    "cargo +nightly fuzz build" \
    "scripts/check_targets.sh" \
    "scripts/check_no_alloc_smoke.sh" \
    "scripts/check_migration_smoke.sh" \
    "scripts/check_simd_feature_bundles.sh" \
    "scripts/check_backend_evidence.sh" \
    "scripts/check_kani.sh" \
    "scripts/generate_ct_asm_evidence.sh" \
    "scripts/generate-sbom.sh" \
    "scripts/reproducible_build_check.sh"
do
    if ! grep -F -q "$required_release_gate_command" scripts/stable_release_gate.sh; then
        echo "release metadata: stable release gate is missing $required_release_gate_command" >&2
        exit 1
    fi
done

for required_trust_text in \
    "Runtime dependencies | Zero external crates" \
    "Active backend | Scalar only" \
    "no formal cryptographic constant-time guarantee" \
    "formally verified cryptographic constant-time behavior" \
    "an active hardware-accelerated backend" \
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
    "Do not lower \`rust-version\` only to make Kani run"
do
    if ! grep -F -q "$required_kani_text" docs/KANI.md; then
        echo "release metadata: Kani policy is missing required text: $required_kani_text" >&2
        exit 1
    fi
done

for required_dependency_review_text in \
    "v0.12 Final Admission Review" \
    "No optional ecosystem integration has a strong enough security and maintenance case" \
    "zeroize\` and \`subtle\` remain deferred"
do
    if ! grep -F -q "$required_dependency_review_text" docs/DEPENDENCIES.md; then
        echo "release metadata: dependency policy is missing required review text: $required_dependency_review_text" >&2
        exit 1
    fi
done

case "$cargo_version" in
    *-*)
        required_readme_simd_status="development remains scalar-only unless that full evidence package lands"
        ;;
    *)
        required_readme_simd_status="release remains scalar-only"
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
    "scripts/validate-msrv-policy.sh" \
    "scripts/check_migration_smoke.sh" \
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

package_list="$(
    cargo package --locked --allow-dirty --list
)"

for required_package_file in \
    "CHANGELOG.md" \
    "CONTRIBUTING.md" \
    "deny.toml" \
    "LICENSE-APACHE" \
    "LICENSE-MIT" \
    "README.md" \
    "rust-toolchain.toml" \
    "SECURITY.md" \
    "docs/API_AUDIT.md" \
    "docs/ASYNC.md" \
    "docs/BENCHMARKS.md" \
    "docs/CONSTANT_TIME.md" \
    "docs/CT_ASM_REVIEW.md" \
    "docs/DEPENDENCIES.md" \
    "docs/DUDECT.md" \
    "docs/FUZZING.md" \
    "docs/INVARIANTS.md" \
    "docs/KANI.md" \
    "docs/MIGRATION.md" \
    "docs/PANIC_POLICY.md" \
    "docs/PLAN.md" \
    "docs/RELEASE.md" \
    "docs/RELEASE_EVIDENCE.md" \
    "docs/SECURITY_CONTROLS.md" \
    "docs/SIMD_ADMISSION.md" \
    "docs/SIMD.md" \
    "docs/TRUST.md" \
    "docs/UNSAFE.md" \
    "portability/no_alloc_smoke/src/lib.rs" \
    "portability/migration_smoke/src/lib.rs" \
    "scripts/check_backend_evidence.sh" \
    "scripts/validate-api-audit.sh" \
    "scripts/check_dudect.sh" \
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
    "scripts/reproducible_build_check.sh" \
    "scripts/stable_release_gate.sh" \
    "scripts/validate-constant-time-policy.sh" \
    "scripts/validate-dependencies.sh" \
    "scripts/validate-doc-versions.sh" \
    "scripts/validate-msrv-policy.sh" \
    "scripts/validate-panic-policy.sh" \
    "scripts/validate-release-metadata.sh" \
    "scripts/validate-simd-admission.sh" \
    "scripts/validate-unsafe-boundary.sh" \
    "src/lib.rs" \
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
