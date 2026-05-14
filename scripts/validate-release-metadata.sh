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
test -s docs/ASYNC.md
test -s docs/BENCHMARKS.md
test -s docs/CONSTANT_TIME.md
test -s docs/DEPENDENCIES.md
test -s docs/FUZZING.md
test -s docs/MIGRATION.md
test -s docs/PANIC_POLICY.md
test -s docs/PLAN.md
test -s docs/RELEASE.md
test -s docs/RELEASE_EVIDENCE.md
test -s docs/SECURITY_CONTROLS.md
test -s docs/SIMD.md
test -s docs/TRUST.md
test -s docs/UNSAFE.md

for required_script in \
    "scripts/check_backend_evidence.sh" \
    "scripts/check_fuzz.sh" \
    "scripts/check_fuzz_corpus.sh" \
    "scripts/check_kani.sh" \
    "scripts/check_miri.sh" \
    "scripts/check_perf.sh" \
    "scripts/check_reserved_features.sh" \
    "scripts/check_simd_feature_bundles.sh" \
    "scripts/check_targets.sh" \
    "scripts/checks.sh" \
    "scripts/ci_install_rust.sh" \
    "scripts/generate-sbom.sh" \
    "scripts/reproducible_build_check.sh" \
    "scripts/stable_release_gate.sh" \
    "scripts/validate-constant-time-policy.sh" \
    "scripts/validate-dependencies.sh" \
    "scripts/validate-panic-policy.sh" \
    "scripts/validate-release-metadata.sh" \
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
    "docs/ASYNC.md" \
    "docs/BENCHMARKS.md" \
    "docs/CONSTANT_TIME.md" \
    "docs/DEPENDENCIES.md" \
    "docs/FUZZING.md" \
    "docs/MIGRATION.md" \
    "docs/PANIC_POLICY.md" \
    "docs/PLAN.md" \
    "docs/RELEASE.md" \
    "docs/RELEASE_EVIDENCE.md" \
    "docs/SECURITY_CONTROLS.md" \
    "docs/SIMD.md" \
    "docs/TRUST.md" \
    "docs/UNSAFE.md" \
    "scripts/check_backend_evidence.sh" \
    "scripts/check_fuzz.sh" \
    "scripts/check_fuzz_corpus.sh" \
    "scripts/check_kani.sh" \
    "scripts/check_miri.sh" \
    "scripts/check_perf.sh" \
    "scripts/check_reserved_features.sh" \
    "scripts/check_simd_feature_bundles.sh" \
    "scripts/check_targets.sh" \
    "scripts/checks.sh" \
    "scripts/ci_install_rust.sh" \
    "scripts/generate-sbom.sh" \
    "scripts/reproducible_build_check.sh" \
    "scripts/stable_release_gate.sh" \
    "scripts/validate-constant-time-policy.sh" \
    "scripts/validate-dependencies.sh" \
    "scripts/validate-panic-policy.sh" \
    "scripts/validate-release-metadata.sh" \
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

echo "release metadata: ok"
