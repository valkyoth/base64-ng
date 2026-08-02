#!/usr/bin/env sh
set -eu

browser="${BASE64_NG_BROWSER:-}"
if [ -z "$browser" ]; then
    for candidate in google-chrome chromium chromium-browser chrome microsoft-edge; do
        if command -v "$candidate" >/dev/null 2>&1; then
            browser="$(command -v "$candidate")"
            break
        fi
    done
fi
if [ -z "$browser" ]; then
    echo "2.0 wasm loader browser: skipping Chromium; set BASE64_NG_BROWSER"
    exit 0
fi

python3 scripts/wasm_loader_browser_smoke.py --browser chromium --binary "$browser"
