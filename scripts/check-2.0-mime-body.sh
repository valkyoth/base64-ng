#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-mime/Cargo.toml"

echo "2.0 MIME body: no-default compile"
cargo check --manifest-path "$manifest" --no-default-features

echo "2.0 MIME body: alloc library tests"
cargo test --manifest-path "$manifest" --no-default-features --features alloc --lib

echo "2.0 MIME body: conformance and interoperability"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 MIME body: lint and docs"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
cargo doc --manifest-path "$manifest" --no-deps --all-features

echo "2.0 MIME body: RFC source lock"
BASE64_NG_RFC_SKIP_PACKAGE=1 scripts/verify-rfcs.sh

echo "2.0 MIME body: scope and package policy"
for required in \
    "RFC 2045 Section 6.8" \
    "does not parse" \
    "MimeBodyDecodePolicy::Rfc2045Compatible" \
    "finite"
do
    if ! grep -F -q "$required" crates/base64-ng-mime/README.md docs/2.0_MIME_BODY.md; then
        echo "2.0 MIME body: missing scope text: $required" >&2
        exit 1
    fi
done

if grep -R -E -n '^pub (struct|enum|trait|type|const|fn) (Mime|MIME)[A-Za-z0-9_]*' \
    crates/base64-ng-mime/src \
    | grep -E -v '(MimeBody|MIME_BODY|mime_content_transfer_body|_mime_content_transfer_body_)'
then
    echo "2.0 MIME body: public MIME name escaped body/content-transfer scope" >&2
    exit 1
fi

python3 - <<'PY'
import json
import subprocess

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--no-deps"
]))
package = next(item for item in metadata["packages"] if item["name"] == "base64-ng-mime")
runtime = sorted(
    dependency["name"] for dependency in package["dependencies"]
    if dependency.get("kind") in (None, "normal")
)
if runtime != ["base64-ng"]:
    raise SystemExit(f"2.0 MIME body: unexpected runtime dependencies: {runtime}")
PY

package_list="$(mktemp)"
trap 'rm -f "$package_list"' EXIT HUP INT TERM
cargo package --locked --allow-dirty --list -p base64-ng-mime >"$package_list"
if grep -E '(^|/)rfc/' "$package_list"; then
    echo "2.0 MIME body: package contains locked RFC source material" >&2
    exit 1
fi

echo "2.0 MIME body: Section 6.8 scope, bounds, interoperability, and package policy ok"
