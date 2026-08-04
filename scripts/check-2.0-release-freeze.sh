#!/usr/bin/env sh
set -eu

version="2.0.0"
snapshot_dir="api-snapshots/v${version}"
evidence_dir="target/release-evidence/commit-54"
package_list_dir="$evidence_dir/package-lists"

packages="
base64-ng
base64-ng-sanitization
base64-ng-derive
base64-ng-imap
base64-ng-mime
base64-ng-multibase
base64-ng-password
base64-ng-openpgp
base64-ng-pem
base64-ng-serde
base64-ng-bytes
base64-ng-subtle
base64-ng-tokio
"

fail() {
    echo "2.0 release freeze: $1" >&2
    exit 1
}

test "$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p')" = "$version" ||
    fail "root package version is not $version"
grep -F -q 'policy = "synced-family"' release-crates.toml ||
    fail "release policy is not synced-family"
[ "$(grep -F -c 'publish = true' release-crates.toml)" -eq 13 ] ||
    fail "synchronized plan does not select exactly 13 Rust packages"
if grep -F -q 'publish = false' release-crates.toml; then
    fail "synchronized plan contains an unpublished Rust package"
fi

scripts/release_crates.py --check

test -d "$snapshot_dir" || fail "missing frozen API snapshot directory"
test ! -e api-snapshots/2.0-development ||
    fail "temporary development API snapshot directory still exists"

mkdir -p "$package_list_dir"
for package in $packages; do
    test -s "$snapshot_dir/$package.txt" ||
        fail "missing frozen API snapshot for $package"
    cargo package --locked --allow-dirty --list -p "$package" \
        >"$package_list_dir/$package.txt"
    test -s "$package_list_dir/$package.txt" ||
        fail "empty package listing for $package"
    if grep -E -q '(^|/)rfc/.*\.txt$|(^|/)spec/.*\.(txt|rst|csv)$' \
        "$package_list_dir/$package.txt"; then
        fail "$package package contains locked source material"
    fi
done

grep -F -q 'base64-ng = "2.0.0"' README.md ||
    fail "README is missing the 2.0.0 dependency"
grep -F -q 'base64-ng = "2.0.0"' docs/MIGRATION.md ||
    fail "migration guide is missing the 2.0.0 dependency"
grep -F -q 'Persistent-provider inventory: **none**.' docs/2.0_RELEASE_FREEZE.md ||
    fail "persistent-provider inventory is not explicit"
test -s release-notes/RELEASE_NOTES_2.0.0.md ||
    fail "missing 2.0.0 release notes"

grep -F -q '"version": "2.0.0"' packages/base64-ng-wasm-loader/package.json ||
    fail "wasm loader package version is not 2.0.0"
grep -F -q 'base64-ng-wasm-provenance-v1' \
    packages/base64-ng-wasm-loader/scripts/build.mjs ||
    fail "wasm build does not emit source provenance"
grep -F -q 'BASE64_NG_SOURCE_COMMIT' scripts/check-2.0-wasm-loader.sh ||
    fail "wasm package gate does not bind the source commit"

scripts/check-2.0-migration-smoke.sh
scripts/check_migration_smoke.sh

if [ "${BASE64_NG_RUN_COMMIT54_PUBLISH_DRY_RUN:-0}" = "1" ]; then
    cargo publish --locked --allow-dirty --dry-run -p base64-ng
else
    echo "2.0 release freeze: core publish dry-run skipped; set BASE64_NG_RUN_COMMIT54_PUBLISH_DRY_RUN=1"
fi

git rev-parse HEAD >"$evidence_dir/source-commit.txt"
rustc -Vv >"$evidence_dir/rustc.txt"
echo "2.0 release freeze: synchronized API, documentation, and package family ok"
