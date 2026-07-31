#!/usr/bin/env sh
set -eu

mode="${1:-}"
if [ -n "$mode" ] && [ "$mode" != "--browsers" ]; then
    echo "2.0 web forgiving: usage: $0 [--browsers]" >&2
    exit 2
fi

doc="docs/2.0_WEB_FORGIVING_BASE64.md"
fixture="tests/fixtures/whatwg-forgiving-base64.txt"
fixture_sha256="ce88dbff1114e3e0a03d88bd1712c8dd33b4d2bd07a25bff6781a7b2388877c4"

for required in \
    '`web::FORGIVING`' \
    '`ForgivingError::InvalidInput`' \
    'not exported from `secret::*`' \
    '`base64` 0.23.0' \
    'Node/V8' \
    'Safari/WebKit'
do
    if ! grep -F -q "$required" "$doc"; then
        echo "2.0 web forgiving: documentation is missing: $required" >&2
        exit 1
    fi
done

test -s "$fixture"
python3 scripts/whatwg_forgiving_fixtures.py >/dev/null
actual_fixture_sha256="$(python3 scripts/whatwg_forgiving_fixtures.py --sha256)"
if [ "$actual_fixture_sha256" != "$fixture_sha256" ]; then
    echo "2.0 web forgiving: fixture corpus changed without reviewed checksum update" >&2
    exit 1
fi

if rg -n -F -e 'web::' -e 'FORGIVING' src/v2/secret.rs src/v2/secret_in_place.rs; then
    echo "2.0 web forgiving: web policy leaked into secret modules" >&2
    exit 1
fi

cargo test --lib 'v2::web_tests' --all-features
cargo check --lib --no-default-features
RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo test \
    --manifest-path perf/Cargo.toml \
    'v2_model::tests::named_compatibility_presets_match_pinned_base64'

grep -F -q 'whatwg-forgiving-base64' scripts/check_wasm_runtime_dispatch.sh
grep -F -q 'whatwg_forgiving_fixtures.py' scripts/check_wasm_browser_dispatch.sh
grep -F -q 'load_fixtures' scripts/wasm_webdriver_smoke.py
grep -F -q 'wasm_webdriver_smoke.py' scripts/check_wasm_browser_firefox_dispatch.sh
grep -F -q 'wasm_webdriver_smoke.py' scripts/check_wasm_browser_safari_dispatch.sh

if [ "$mode" = "--browsers" ]; then
    scripts/check_wasm_runtime_dispatch.sh
    scripts/check_wasm_browser_dispatch.sh
    scripts/check_wasm_browser_firefox_dispatch.sh
    scripts/check_wasm_browser_safari_dispatch.sh
fi

echo "2.0 web forgiving: exact web and distinct expert policies ok"
