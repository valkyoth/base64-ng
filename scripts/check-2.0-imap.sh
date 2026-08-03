#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-imap/Cargo.toml"
msrv_toolchain="${BASE64_NG_MSRV_TOOLCHAIN:-1.90.0}"

echo "2.0 IMAP payload: locked RFC, errata, and requirement mappings"
BASE64_NG_RFC_SKIP_PACKAGE=1 scripts/verify-rfcs.sh

echo "2.0 IMAP payload: no-default compile"
cargo check --manifest-path "$manifest" --no-default-features

echo "2.0 IMAP payload: allocated conformance and interoperability"
cargo test --manifest-path "$manifest" --no-default-features --features alloc

echo "2.0 IMAP payload: all-feature conformance"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 IMAP payload: lint, docs, and fuzz target"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
cargo doc --manifest-path "$manifest" --no-deps --all-features
cargo check --manifest-path fuzz/Cargo.toml --bin imap_payload

if rustup run "$msrv_toolchain" rustc --version >/dev/null 2>&1; then
    echo "2.0 IMAP payload: optional local MSRV evidence ($msrv_toolchain)"
    cargo +"$msrv_toolchain" check --manifest-path "$manifest" --no-default-features
    cargo +"$msrv_toolchain" check --manifest-path "$manifest" --all-features
else
    echo "2.0 IMAP payload: skipping local MSRV checks; Rust $msrv_toolchain is not installed"
    echo "2.0 IMAP payload: the dedicated CI MSRV matrix remains authoritative"
fi

echo "2.0 IMAP payload: dependency and package scope"
scripts/cargo-deny-check.sh "$manifest" deny.toml
package_list="$(mktemp)"
trap 'rm -f "$package_list"' EXIT HUP INT TERM
cargo package --locked --allow-dirty --list -p base64-ng-imap >"$package_list"
if grep -E '(^|/)rfc/' "$package_list"; then
    echo "2.0 IMAP payload: package contains locked RFC material" >&2
    exit 1
fi
for required in \
    "README.md" \
    "src/incremental.rs" \
    "src/lib.rs" \
    "tests/imap_payload.rs" \
    "tests/interoperability.rs"
do
    if ! grep -F -x -q "$required" "$package_list"; then
        echo "2.0 IMAP payload: package is missing $required" >&2
        exit 1
    fi
done

for required in \
    "not a complete IMAP modified UTF-7 mailbox codec" \
    "already converted to UTF-16BE" \
    "RFC 3501 is obsolete" \
    "ordinary public-data API"
do
    if ! grep -F -q "$required" crates/base64-ng-imap/README.md docs/2.0_IMAP.md; then
        echo "2.0 IMAP payload: missing scope text: $required" >&2
        exit 1
    fi
done

for required in \
    "IMAP_MUTF7_ALPHABET_NO_PAD" \
    "InvalidUtf16BeLength" \
    "modified_utf7_payload_decoded_len"
do
    if ! grep -R -F -q "$required" crates/base64-ng-imap/src; then
        echo "2.0 IMAP payload: missing implementation marker: $required" >&2
        exit 1
    fi
done

python3 - <<'PY'
import json
import subprocess

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--no-deps"
]))
package = next(
    item for item in metadata["packages"]
    if item["name"] == "base64-ng-imap"
)
runtime = sorted(
    dependency["name"] for dependency in package["dependencies"]
    if dependency.get("kind") in (None, "normal")
)
if runtime != ["base64-ng"]:
    raise SystemExit(f"2.0 IMAP payload: unexpected runtime dependencies: {runtime}")
PY

echo "2.0 IMAP payload: scope, bounds, canonicality, interoperability, and package policy ok"
