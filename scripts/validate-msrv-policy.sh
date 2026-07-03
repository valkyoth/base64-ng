#!/usr/bin/env sh
set -eu

cargo_rust_version="$(
    sed -n 's/^rust-version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"
toolchain_version="$(
    sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml | sed -n '1p'
)"

if [ -z "$cargo_rust_version" ]; then
    echo "MSRV policy: Cargo.toml rust-version is missing" >&2
    exit 1
fi

if [ -z "$toolchain_version" ]; then
    echo "MSRV policy: rust-toolchain.toml channel is missing" >&2
    exit 1
fi

msrv_full="$cargo_rust_version.0"

case "$toolchain_version" in
    *-*)
        echo "MSRV policy: rust-toolchain.toml must pin a stable release toolchain, got $toolchain_version" >&2
        exit 1
        ;;
esac

if ! grep -F -q "$msrv_full" .github/workflows/ci.yml; then
    echo "MSRV policy: CI compatibility matrix is missing MSRV $msrv_full" >&2
    exit 1
fi

if ! grep -F -q '[package.metadata.docs.rs]' Cargo.toml; then
    echo "MSRV policy: Cargo.toml is missing docs.rs metadata" >&2
    exit 1
fi

if ! grep -F -q 'all-features = true' Cargo.toml; then
    echo "MSRV policy: docs.rs metadata must build all features" >&2
    exit 1
fi

for required_doc in README.md docs/TRUST.md docs/PLAN.md docs/KANI.md; do
    if ! grep -F -q "\`$msrv_full\`" "$required_doc"; then
        echo "MSRV policy: $required_doc does not mention \`$msrv_full\`" >&2
        exit 1
    fi
done

if ! grep -F -q "active release toolchain" docs/RELEASE.md README.md; then
    echo "MSRV policy: release docs must explain the active release toolchain" >&2
    exit 1
fi

if ! grep -F -q "MSRV remains Rust \`$msrv_full\`" docs/RELEASE.md README.md; then
    echo "MSRV policy: release docs must explain that MSRV remains Rust $msrv_full" >&2
    exit 1
fi

if ! grep -F -q 'run: scripts/ci_install_rust.sh' .github/workflows/ci.yml; then
    echo "MSRV policy: CI must install Rust through scripts/ci_install_rust.sh" >&2
    exit 1
fi

for target in \
    "x86_64-unknown-linux-gnu" \
    "aarch64-unknown-linux-gnu" \
    "x86_64-unknown-freebsd" \
    "wasm32-unknown-unknown" \
    "thumbv7em-none-eabihf"
do
    if ! grep -F -q "$target" .github/workflows/ci.yml; then
        echo "MSRV policy: CI target matrix is missing $target" >&2
        exit 1
    fi

    if ! grep -F -q "$target" docs/RELEASE_EVIDENCE.md; then
        echo "MSRV policy: release evidence docs are missing $target" >&2
        exit 1
    fi
done

for required_gate in \
    "scripts/check_miri.sh" \
    "scripts/check_kani.sh" \
    "scripts/check_fuzz.sh" \
    "scripts/generate-sbom.sh" \
    "scripts/reproducible_build_check.sh"
do
    if ! grep -F -q "$required_gate" scripts/stable_release_gate.sh scripts/checks.sh; then
        echo "MSRV policy: local gates are missing $required_gate" >&2
        exit 1
    fi
done

echo "MSRV policy: ok (MSRV $msrv_full, active $toolchain_version)"
