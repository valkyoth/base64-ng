#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-pem/Cargo.toml"

echo "2.0 PEM: no-default no_std + alloc compile"
cargo check --manifest-path "$manifest" --no-default-features

echo "2.0 PEM: complete grammar, RFC vector, interoperability, and secrets"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 PEM: lint and docs"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 PEM: RFC source lock"
BASE64_NG_RFC_SKIP_PACKAGE=1 scripts/verify-rfcs.sh

echo "2.0 PEM: scope and cleanup policy"
for required in \
    "complete textual encoding grammar" \
    "does not parse ASN.1" \
    "Legacy OpenSSL encapsulated headers" \
    "parse_pem_secret_block" \
    "finite"
do
    if ! grep -F -q "$required" crates/base64-ng-pem/README.md docs/2.0_PEM.md; then
        echo "2.0 PEM: missing scope text: $required" >&2
        exit 1
    fi
done

for required_source in \
    "base64_ng::secure_wipe" \
    "SecretVecFrame" \
    "PemParsePolicy::Strict" \
    "LegacyHeadersNotSupported" \
    "noncanonical_boundary_lines"
do
    if ! grep -R -F -q "$required_source" crates/base64-ng-pem/src; then
        echo "2.0 PEM: missing implementation boundary: $required_source" >&2
        exit 1
    fi
done

if ! grep -F -q "struct Lines<'a>" crates/base64-ng-pem/src/parser/lines.rs; then
    echo "2.0 PEM: missing cursor-based physical-line scanner" >&2
    exit 1
fi

if grep -F -q "Vec<Line" crates/base64-ng-pem/src/parser.rs || \
    grep -F -q "body_lengths" crates/base64-ng-pem/src/parser.rs
then
    echo "2.0 PEM: document-wide line metadata allocation returned" >&2
    exit 1
fi

if grep -R -n -E 'Proc-Type|DEK-Info' crates/base64-ng-pem/src \
    | grep -v 'LegacyHeadersNotSupported'
then
    echo "2.0 PEM: legacy encapsulated header support escaped exclusion policy" >&2
    exit 1
fi

python3 - <<'PY'
import json
import subprocess

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--no-deps"
]))
package = next(item for item in metadata["packages"] if item["name"] == "base64-ng-pem")
runtime = sorted(
    dependency["name"] for dependency in package["dependencies"]
    if dependency.get("kind") in (None, "normal")
)
if runtime != ["base64-ng"]:
    raise SystemExit(f"2.0 PEM: unexpected runtime dependencies: {runtime}")
PY

package_list="$(mktemp)"
trap 'rm -f "$package_list"' EXIT HUP INT TERM
cargo package --locked --allow-dirty --list -p base64-ng-pem >"$package_list"
if grep -E '(^|/)rfc/' "$package_list"; then
    echo "2.0 PEM: package contains locked RFC source material" >&2
    exit 1
fi

echo "2.0 PEM: RFC 7468 grammar, bounds, cleanup, interoperability, and package policy ok"
