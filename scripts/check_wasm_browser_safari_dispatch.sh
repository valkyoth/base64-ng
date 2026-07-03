#!/usr/bin/env sh
set -eu

wasm_target="${1:-wasm32-unknown-unknown}"

if ! rustup target list --installed 2>/dev/null | grep -F -x -q "$wasm_target"; then
    echo "wasm Safari dispatch: skipping $wasm_target; Rust target is not installed"
    exit 0
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "wasm Safari dispatch: skipping Safari smoke; python3 is not installed"
    exit 0
fi

driver="${BASE64_NG_SAFARIDRIVER:-}"
if [ -z "$driver" ]; then
    if command -v safaridriver >/dev/null 2>&1; then
        driver="$(command -v safaridriver)"
    elif [ -x "/usr/bin/safaridriver" ]; then
        driver="/usr/bin/safaridriver"
    else
        echo "wasm Safari dispatch: skipping Safari smoke; set BASE64_NG_SAFARIDRIVER=/path/to/safaridriver"
        exit 0
    fi
fi

python3 scripts/wasm_webdriver_smoke.py \
    --browser safari \
    --driver "$driver" \
    --target "$wasm_target" \
    --no-headless
