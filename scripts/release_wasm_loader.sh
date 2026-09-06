#!/usr/bin/env sh
set -eu

mode="${1:-check}"
case "$mode" in
    check | dry-run | publish) ;;
    *)
        echo "usage: scripts/release_wasm_loader.sh [check|dry-run|publish]" >&2
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
scripts/check-2.0-wasm-loader.sh

if [ "$mode" = "check" ]; then
    if [ "$npm_selected" = "true" ]; then
        echo "wasm loader release: selected package $npm_version passed checks"
    else
        echo "wasm loader release: unchanged package $npm_version checked during Rust $rust_version release"
    fi
    exit 0
fi

if [ -n "$(git status --porcelain --untracked-files=all)" ]; then
    echo "wasm loader release: refusing publication from a dirty worktree" >&2
    exit 1
fi

tag="v$rust_version"
tagged="$(git rev-list -n 1 "$tag" 2>/dev/null || true)"
if [ "$head" != "$tagged" ]; then
    echo "wasm loader release: HEAD is not tagged as $tag" >&2
    exit 1
fi
scripts/verify-release-tag.sh "$tag"

if [ "$mode" = "dry-run" ]; then
    (cd "$package_dir" && npm publish --dry-run)
    echo "wasm loader release: dry-run passed for $tag"
    exit 0
fi

(cd "$package_dir" && npm publish --provenance)

echo "wasm loader release: published $npm_name@$npm_version from verified $tag"
