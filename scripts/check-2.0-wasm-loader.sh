#!/usr/bin/env sh
set -eu

package_dir="packages/base64-ng-wasm-loader"
evidence_dir="target/release-evidence/wasm-loader"
install_dir="target/wasm-loader-package"
pack_dir="$install_dir/packed"
package_extract="$install_dir/package"
npm_cache="target/npm-cache"
alternate_root="$evidence_dir/path-independent/repository"
source_commit="$(git rev-parse HEAD)"
export BASE64_NG_SOURCE_COMMIT="$source_commit"
export npm_config_cache="$npm_cache"

if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
    echo "2.0 wasm loader: skipping JavaScript package; Node and npm are required"
    exit 0
fi
if ! command -v rustup >/dev/null 2>&1; then
    echo "2.0 wasm loader: rustup is required to build the wasm artifacts" >&2
    exit 1
fi
if ! rustup target list --installed 2>/dev/null | grep -F -x -q wasm32-unknown-unknown; then
    echo "2.0 wasm loader: installing missing Rust target wasm32-unknown-unknown"
    rustup target add wasm32-unknown-unknown
fi

mkdir -p "$evidence_dir" "$pack_dir" "$package_extract" "$npm_cache"
rm -f "$pack_dir"/*.tgz
rm -rf "$package_extract"
mkdir -p "$package_extract"

echo "2.0 wasm loader: format and clippy"
if ! grep -F -q '#![deny(unsafe_code)]' "$package_dir/wasm/src/lib.rs"; then
    echo "2.0 wasm loader: artifact crate must deny unsafe code by default" >&2
    exit 1
fi
allow_count="$(grep -c '^#\[allow(unsafe_code)\]$' "$package_dir/wasm/src/lib.rs")"
if [ "$allow_count" -ne 25 ]; then
    echo "2.0 wasm loader: expected exactly 25 reviewed unsafe ABI sites" >&2
    exit 1
fi
if ! grep -F -q 'pub extern "C" fn base64_ng_clear_used' "$package_dir/wasm/src/lib.rs"; then
    echo "2.0 wasm loader: tracked-range cleanup export is missing" >&2
    exit 1
fi
if grep -F -q 'core::hint::spin_loop()' "$package_dir/wasm/src/lib.rs" \
    || ! grep -F -q 'core::arch::wasm32::unreachable()' "$package_dir/wasm/src/lib.rs"; then
    echo "2.0 wasm loader: panic handler must trap instead of spinning" >&2
    exit 1
fi
cargo fmt --manifest-path "$package_dir/wasm/Cargo.toml" -- --check
CARGO_TARGET_DIR="$package_dir/target-scalar" \
    cargo clippy --locked --manifest-path "$package_dir/wasm/Cargo.toml" \
        --target wasm32-unknown-unknown --release -- -D warnings
CARGO_TARGET_DIR="$package_dir/target-simd128" \
RUSTFLAGS='-C target-feature=+simd128' \
    cargo clippy --locked --manifest-path "$package_dir/wasm/Cargo.toml" \
        --target wasm32-unknown-unknown --release --features simd -- -D warnings

echo "2.0 wasm loader: deterministic scalar and simd128 artifacts"
(cd "$package_dir" && npm ci --ignore-scripts && npm run build)
if (cd "$package_dir" && \
    BASE64_NG_SOURCE_COMMIT=0000000000000000000000000000000000000000 \
    npm run build >/dev/null 2>&1)
then
    echo "2.0 wasm loader: build accepted provenance that did not match HEAD" >&2
    exit 1
fi
(cd "$package_dir/artifacts" && sha256sum -c SHA256SUMS)
package_version="$(node -p "require('./$package_dir/package.json').version")"
python3 - "$package_dir/artifacts/PROVENANCE.json" "$source_commit" "$package_version" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    provenance = json.load(handle)
if provenance.get("schema") != "base64-ng-wasm-provenance-v1":
    raise SystemExit("2.0 wasm loader: invalid provenance schema")
if provenance.get("package") != "@valkyoth/base64-ng-wasm-loader" or provenance.get("version") != sys.argv[3]:
    raise SystemExit("2.0 wasm loader: provenance package identity mismatch")
if provenance.get("sourceCommit") != sys.argv[2]:
    raise SystemExit("2.0 wasm loader: provenance source commit mismatch")
PY
while read -r digest artifact; do
    if ! grep -F -q "$digest" "$package_dir/src/index.js"; then
        echo "2.0 wasm loader: embedded digest is missing for $artifact" >&2
        exit 1
    fi
done <"$package_dir/artifacts/SHA256SUMS"
sha256sum "$package_dir"/artifacts/*.wasm >"$evidence_dir/artifacts-first.sha256"
(cd "$package_dir" && npm run build)
sha256sum "$package_dir"/artifacts/*.wasm >"$evidence_dir/artifacts-second.sha256"
cmp "$evidence_dir/artifacts-first.sha256" "$evidence_dir/artifacts-second.sha256"

echo "2.0 wasm loader: path-independent artifact rebuild"
rm -rf "$alternate_root"
mkdir -p "$alternate_root"
git ls-files --cached --others --exclude-standard | while IFS= read -r tracked_file; do
    if [ ! -f "$tracked_file" ]; then
        continue
    fi
    mkdir -p "$alternate_root/$(dirname "$tracked_file")"
    cp "$tracked_file" "$alternate_root/$tracked_file"
done
(cd "$alternate_root/$package_dir" && \
    BASE64_NG_ALLOW_SOURCE_COMMIT_WITHOUT_GIT=1 npm run build)
sha256sum "$alternate_root/$package_dir"/artifacts/*.wasm \
    | awk '{ print $1 }' >"$evidence_dir/artifacts-alternate-path.sha256"
sha256sum "$package_dir"/artifacts/*.wasm \
    | awk '{ print $1 }' >"$evidence_dir/artifacts-current-path.sha256"
cmp "$evidence_dir/artifacts-current-path.sha256" \
    "$evidence_dir/artifacts-alternate-path.sha256"

for artifact in "$package_dir"/artifacts/*.wasm; do
    if LC_ALL=C grep -a -E -q '/home/|/Users/|/workspace/|/builds/|[A-Za-z]:\\\\' "$artifact"; then
        echo "2.0 wasm loader: absolute checkout path leaked into $artifact" >&2
        exit 1
    fi
done

echo "2.0 wasm loader: Node/V8 differential and hostile-input tests"
(cd "$package_dir" && npm test)
(cd "$package_dir" && node test/benchmark.mjs) >"$evidence_dir/node-benchmark.json"
cat "$evidence_dir/node-benchmark.json"

if command -v wasmtime >/dev/null 2>&1; then
    echo "2.0 wasm loader: Wasmtime scalar and simd128 self-tests"
    wasmtime run -C cache=n --invoke base64_ng_self_test \
        "$package_dir/artifacts/base64-ng-scalar.wasm" >/dev/null
    wasmtime run -C cache=n --invoke base64_ng_self_test \
        "$package_dir/artifacts/base64-ng-simd128.wasm" >/dev/null
else
    echo "2.0 wasm loader: skipping Wasmtime self-tests; wasmtime is not installed"
fi

echo "2.0 wasm loader: exact npm package and install smoke"
tarball_name="$(cd "$package_dir" && npm pack --ignore-scripts --silent --pack-destination "../../$pack_dir")"
tarball="$pack_dir/$tarball_name"
test -s "$tarball"
tar -xzf "$tarball" -C "$install_dir"
test -s "$package_extract/src/index.js"
test -s "$package_extract/src/index.d.ts"
test -s "$package_extract/artifacts/base64-ng-scalar.wasm"
test -s "$package_extract/artifacts/base64-ng-simd128.wasm"
test -s "$package_extract/artifacts/SHA256SUMS"
test -s "$package_extract/artifacts/PROVENANCE.json"
(cd "$package_extract/artifacts" && sha256sum -c SHA256SUMS)
cmp "$package_dir/artifacts/PROVENANCE.json" \
    "$package_extract/artifacts/PROVENANCE.json"
if tar -tzf "$tarball" | grep -E -q '(^|/)(test|scripts|wasm|target-|package-lock\.json)'; then
    echo "2.0 wasm loader: npm artifact contains development-only files" >&2
    exit 1
fi
node scripts/wasm_loader_install_smoke.mjs "$package_extract"
tar -tzf "$tarball" | sort >"$evidence_dir/npm-package-files.txt"
sha256sum "$tarball" >"$evidence_dir/npm-package.sha256"

cp tests/wasm-loader-browser-smoke.html "$install_dir/browser-smoke.html"
cp tests/wasm-loader-browser-smoke.mjs "$install_dir/browser-smoke.mjs"

echo "2.0 wasm loader: ok"
