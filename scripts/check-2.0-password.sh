#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-password/Cargo.toml"
msrv_toolchain="${BASE64_NG_MSRV_TOOLCHAIN:-1.90.0}"

echo "2.0 password records: no-default compile"
cargo check --manifest-path "$manifest" --no-default-features

echo "2.0 password records: exact formats and interoperability"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 password records: lint, docs, and fuzz target"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
cargo doc --manifest-path "$manifest" --no-deps --all-features
cargo check --manifest-path fuzz/Cargo.toml --bin password_records

if rustup run "$msrv_toolchain" rustc --version >/dev/null 2>&1; then
    echo "2.0 password records: optional local MSRV evidence ($msrv_toolchain)"
    cargo +"$msrv_toolchain" check --manifest-path "$manifest" --no-default-features
    cargo +"$msrv_toolchain" check --manifest-path "$manifest" --all-features
else
    echo "2.0 password records: skipping local MSRV checks; Rust $msrv_toolchain is not installed"
    echo "2.0 password records: the dedicated CI MSRV matrix remains authoritative"
fi

echo "2.0 password records: dependency and package scope"
scripts/cargo-deny-check.sh "$manifest" deny.toml
package_list="$(mktemp)"
trap 'rm -f "$package_list"' EXIT HUP INT TERM
cargo package --locked --allow-dirty --list -p base64-ng-password >"$package_list"
for required in \
    "README.md" \
    "src/lib.rs" \
    "src/pbkdf2.rs" \
    "src/sha_crypt.rs" \
    "tests/interoperability.rs" \
    "tests/password_records.rs"
do
    if ! grep -F -x -q "$required" "$package_list"; then
        echo "2.0 password records: package is missing $required" >&2
        exit 1
    fi
done

for forbidden in \
    "fn hash_password" \
    "fn verify_password" \
    "fn derive_pbkdf2"
do
    if grep -R -F -q "$forbidden" crates/base64-ng-password/src; then
        echo "2.0 password records: forbidden password-computation surface: $forbidden" >&2
        exit 1
    fi
done

for required in \
    "does not accept passwords" \
    "performs no password hashing" \
    "field-selective redacted"
do
    if ! grep -R -i -F -q "$required" crates/base64-ng-password/README.md docs/2.0_PASSWORD_RECORDS.md crates/base64-ng-password/src; then
        echo "2.0 password records: missing scope text: $required" >&2
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
    if item["name"] == "base64-ng-password"
)
runtime = sorted(
    dependency["name"] for dependency in package["dependencies"]
    if dependency.get("kind") in (None, "normal")
)
if runtime != ["base64-ng"]:
    raise SystemExit(f"2.0 password records: unexpected runtime dependencies: {runtime}")
PY

echo "2.0 password records: grammar, permutations, limits, redaction, and package policy ok"
