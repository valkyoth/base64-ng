#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-openpgp/Cargo.toml"
msrv_toolchain="${BASE64_NG_MSRV_TOOLCHAIN:-1.90.0}"
gpg="${GPG:-gpg}"
sq="${SQ:-sq}"

sq_has_dearmor() {
    "$1" packet dearmor --help >/dev/null 2>&1 ||
        "$1" toolbox dearmor --help >/dev/null 2>&1 ||
        "$1" dearmor --help >/dev/null 2>&1
}

echo "2.0 OpenPGP: no-default no_std + alloc compile"
cargo check --manifest-path "$manifest" --no-default-features

echo "2.0 OpenPGP: complete armor, checksum, streaming, and secrets"
cargo test --manifest-path "$manifest" --all-features

if command -v "$gpg" >/dev/null 2>&1 &&
    command -v "$sq" >/dev/null 2>&1 &&
    sq_has_dearmor "$sq"
then
    echo "2.0 OpenPGP: required GnuPG and Sequoia differential evidence"
    GPG="$gpg" SQ="$sq" \
    BASE64_NG_REQUIRE_OPENPGP_INTEROP=1 \
        cargo test --manifest-path "$manifest" --all-features \
        --test interoperability
elif [ -n "${BASE64_NG_REQUIRE_OPENPGP_INTEROP:-}" ]; then
    echo "2.0 OpenPGP: required GnuPG/Sequoia dearmor tooling is unavailable" >&2
    exit 1
else
    echo "2.0 OpenPGP: local external interop skipped; install GnuPG and Sequoia sq"
    echo "2.0 OpenPGP: CI with BASE64_NG_REQUIRE_OPENPGP_INTEROP=1 is authoritative"
fi

echo "2.0 OpenPGP: lint, docs, MSRV, and fuzz target"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
cargo doc --manifest-path "$manifest" --no-deps --all-features
cargo check --manifest-path fuzz/Cargo.toml --bin openpgp_armor
if rustup run "$msrv_toolchain" rustc --version >/dev/null 2>&1; then
    cargo +"$msrv_toolchain" check --manifest-path "$manifest" --no-default-features
    cargo +"$msrv_toolchain" check --manifest-path "$manifest" --all-features
else
    echo "2.0 OpenPGP: skipping local MSRV; dedicated CI remains authoritative"
fi

echo "2.0 OpenPGP: RFC source, dependency, and package scope"
BASE64_NG_RFC_SKIP_PACKAGE=1 scripts/verify-rfcs.sh
scripts/cargo-deny-check.sh "$manifest" deny.toml

for required in \
    "ChecksumPolicy::Rfc9580" \
    "RequireValidCrc24" \
    "parse_secret_armor_block" \
    "cleartext signature" \
    "finite"
do
    if ! grep -R -F -q "$required" crates/base64-ng-openpgp docs/2.0_OPENPGP.md; then
        echo "2.0 OpenPGP: missing policy text: $required" >&2
        exit 1
    fi
done

for source_boundary in \
    "base64_ng::secure_wipe" \
    "SecretVecFrame" \
    "ChecksumStatus::Mismatch" \
    "BodyLineTooLong" \
    "max_total_header_bytes"
do
    if ! grep -R -F -q "$source_boundary" crates/base64-ng-openpgp/src; then
        echo "2.0 OpenPGP: missing implementation boundary: $source_boundary" >&2
        exit 1
    fi
done

package_list="$(mktemp)"
trap 'rm -f "$package_list"' EXIT HUP INT TERM
cargo package --locked --allow-dirty --list -p base64-ng-openpgp >"$package_list"
for required in \
    "README.md" \
    "src/crc24.rs" \
    "src/parser.rs" \
    "src/generator.rs" \
    "tests/interoperability.rs" \
    "tests/openpgp.rs" \
    "tests/std_io.rs"
do
    if ! grep -F -x -q "$required" "$package_list"; then
        echo "2.0 OpenPGP: package is missing $required" >&2
        exit 1
    fi
done
if grep -E '(^|/)rfc/' "$package_list"; then
    echo "2.0 OpenPGP: package contains locked RFC source material" >&2
    exit 1
fi

echo "2.0 OpenPGP: RFC 9580 armor grammar, CRC-24, bounds, and evidence ok"
