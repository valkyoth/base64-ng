#!/usr/bin/env sh
set -eu

if [ ! -s target/wasm-loader-package/package/src/index.js ]; then
    echo "2.0 wasm loader browser: exact package is missing; run scripts/check-2.0-wasm-loader.sh first" >&2
    exit 1
fi

driver="${BASE64_NG_GECKODRIVER:-}"
if [ -z "$driver" ] && command -v geckodriver >/dev/null 2>&1; then
    driver="$(command -v geckodriver)"
fi
if [ -z "$driver" ] && [ -x "$HOME/.cargo/bin/geckodriver" ]; then
    driver="$HOME/.cargo/bin/geckodriver"
fi
if [ -z "$driver" ]; then
    echo "2.0 wasm loader browser: skipping Firefox; set BASE64_NG_GECKODRIVER"
    exit 0
fi

python3 scripts/wasm_loader_browser_smoke.py --browser firefox --driver "$driver"
