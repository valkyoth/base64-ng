#!/usr/bin/env sh
set -eu

if [ ! -s target/wasm-loader-package/package/src/index.js ]; then
    echo "2.0 wasm loader browser: exact package is missing; run scripts/check-2.0-wasm-loader.sh first" >&2
    exit 1
fi

driver="${BASE64_NG_SAFARIDRIVER:-}"
if [ -z "$driver" ] && command -v safaridriver >/dev/null 2>&1; then
    driver="$(command -v safaridriver)"
fi
if [ -z "$driver" ] && [ -x /usr/bin/safaridriver ]; then
    driver=/usr/bin/safaridriver
fi
if [ -z "$driver" ]; then
    echo "2.0 wasm loader browser: skipping Safari; set BASE64_NG_SAFARIDRIVER"
    exit 0
fi

python3 scripts/wasm_loader_browser_smoke.py \
    --browser safari --driver "$driver" --no-headless
