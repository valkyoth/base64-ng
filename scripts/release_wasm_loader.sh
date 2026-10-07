#!/usr/bin/env sh
set -eu

mode="${1:-check}"
case "$mode" in
    check | dry-run | publish | publish-desktop) ;;
    *)
        echo "usage: scripts/release_wasm_loader.sh [check|dry-run|publish|publish-desktop]" >&2
        exit 2
        ;;
esac

package_dir="packages/base64-ng-wasm-loader"
rust_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p')"

if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
    echo "wasm loader release: Node.js and npm are required" >&2
    exit 1
fi

npm_version="$(node -p "require('./$package_dir/package.json').version")"
npm_name="$(node -p "require('./$package_dir/package.json').name")"
npm_access="$(node -p "require('./$package_dir/package.json').publishConfig?.access ?? ''")"
release_plan="$(scripts/release_crates.py --npm-plan)"
planned_rust_version="$(printf '%s\n' "$release_plan" | sed -n 's/^release=//p')"
planned_npm_name="$(printf '%s\n' "$release_plan" | sed -n 's/^name=//p')"
planned_npm_version="$(printf '%s\n' "$release_plan" | sed -n 's/^version=//p')"
npm_selected="$(printf '%s\n' "$release_plan" | sed -n 's/^publish=//p')"
if [ "$npm_name" != "@valkyoth/base64-ng-wasm-loader" ]; then
    echo "wasm loader release: unexpected npm package identity $npm_name" >&2
    exit 1
fi
if [ "$rust_version" != "$planned_rust_version" ]; then
    echo "wasm loader release: Rust package version does not match release-crates.toml" >&2
    exit 1
fi
if [ "$npm_access" != "public" ]; then
    echo "wasm loader release: scoped npm package must publish with public access" >&2
    exit 1
fi
if [ "$npm_name" != "$planned_npm_name" ] || [ "$npm_version" != "$planned_npm_version" ]; then
    echo "wasm loader release: package identity or version does not match release-crates.toml" >&2
    exit 1
fi
if [ "$npm_selected" != "true" ] && [ "$npm_selected" != "false" ]; then
    echo "wasm loader release: invalid npm publication decision in release-crates.toml" >&2
    exit 1
fi
if [ "$mode" != "check" ] && [ "$npm_selected" != "true" ]; then
    echo "wasm loader release: package $npm_version is not selected for the Rust $rust_version release" >&2
    exit 1
fi
head="$(git rev-parse --verify HEAD)"
export BASE64_NG_SOURCE_COMMIT="$head"
tag="v$rust_version"
verify_source() {
    if [ -n "$(git status --porcelain --untracked-files=all)" ]; then
        echo "wasm loader release: refusing publication from a dirty worktree" >&2
        exit 1
    fi
    if [ "$head" != "$(git rev-parse --verify HEAD)" ] ||
        [ "$head" != "$(git rev-list -n 1 "$tag")" ]; then
        echo "wasm loader release: HEAD changed or is not tagged as $tag" >&2
        exit 1
    fi
    scripts/verify-release-tag.sh "$tag"
}
if [ "$mode" != "check" ]; then
    verify_source
    umask 077
    mkdir -p target
    release_dir="$(mktemp -d "$PWD/target/npm-release.XXXXXX")"
    trap 'rm -rf "$release_dir"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    export BASE64_NG_WASM_INSTALL_DIR="$release_dir"
fi
scripts/check-2.0-wasm-loader.sh

if [ "$mode" = "check" ]; then
    if [ "$npm_selected" = "true" ]; then
        echo "wasm loader release: selected package $npm_version passed checks"
    else
        echo "wasm loader release: unchanged package $npm_version checked during Rust $rust_version release"
    fi
    exit 0
fi

# Publish the very archive extracted and tested by the gate, never rebuild it.
tarball="$release_dir/packed/valkyoth-base64-ng-wasm-loader-$npm_version.tgz"
test -s "$tarball"
verify_source
(cd "$release_dir/packed" && sha256sum -c checked.sha256)

if [ "$mode" = "dry-run" ]; then
    npm publish --dry-run --ignore-scripts --access public "$tarball"
    echo "wasm loader release: dry-run passed for $tag"
    exit 0
fi

if [ "$mode" = "publish-desktop" ]; then
    npm publish --provenance=false --ignore-scripts --access public "$tarball"
else
    npm publish --provenance --ignore-scripts --access public "$tarball"
fi

echo "wasm loader release: published $npm_name@$npm_version from verified $tag"
