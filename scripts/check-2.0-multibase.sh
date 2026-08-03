#!/usr/bin/env sh
set -eu

manifest="crates/base64-ng-multibase/Cargo.toml"

echo "2.0 multibase: pinned registry and official vectors"
scripts/validate-multibase-spec.py
scripts/check-multibase-source-mutations.py

for fixture in basic.csv leading_zero.csv two_leading_zeros.csv; do
    cmp "spec/multibase/tests/$fixture" \
        "crates/base64-ng-multibase/tests/fixtures/$fixture"
done

echo "2.0 multibase: no-default conformance"
cargo test --manifest-path "$manifest" --no-default-features

echo "2.0 multibase: alloc conformance"
cargo test --manifest-path "$manifest" --no-default-features --features alloc

echo "2.0 multibase: all-feature conformance and Python interoperability"
cargo test --manifest-path "$manifest" --all-features

echo "2.0 multibase: lint, docs, MSRV, and fuzz target"
cargo clippy --manifest-path "$manifest" --all-targets --all-features -- -D warnings
cargo doc --manifest-path "$manifest" --no-deps --all-features
cargo +1.90.0 check --manifest-path "$manifest" --no-default-features
cargo +1.90.0 check --manifest-path "$manifest" --all-features
cargo check --manifest-path fuzz/Cargo.toml --bin multibase_family

echo "2.0 multibase: dependency and package scope"
scripts/cargo-deny-check.sh "$manifest" deny.toml
package_list="$(mktemp)"
trap 'rm -f "$package_list"' EXIT HUP INT TERM
cargo package --locked --allow-dirty --list -p base64-ng-multibase >"$package_list"
if grep -E '(^|/)spec/multibase/' "$package_list"; then
    echo "2.0 multibase: package contains pinned upstream source material" >&2
    exit 1
fi
for required in \
    "README.md" \
    "src/lib.rs" \
    "tests/fixtures/basic.csv" \
    "tests/fixtures/leading_zero.csv" \
    "tests/fixtures/two_leading_zeros.csv"
do
    if ! grep -F -x -q "$required" "$package_list"; then
        echo "2.0 multibase: package is missing $required" >&2
        exit 1
    fi
done

for required in \
    "not a complete multibase implementation" \
    "Base64MultibaseLimits" \
    "ordinary public-data transforms"
do
    if ! grep -F -q "$required" crates/base64-ng-multibase/README.md docs/2.0_MULTIBASE.md; then
        echo "2.0 multibase: missing scope text: $required" >&2
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
    if item["name"] == "base64-ng-multibase"
)
runtime = sorted(
    dependency["name"] for dependency in package["dependencies"]
    if dependency.get("kind") in (None, "normal")
)
if runtime != ["base64-ng"]:
    raise SystemExit(f"2.0 multibase: unexpected runtime dependencies: {runtime}")
PY

echo "2.0 multibase: Base64-family scope, bounds, interoperability, and package policy ok"
