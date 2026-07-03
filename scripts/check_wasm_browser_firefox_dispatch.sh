#!/usr/bin/env sh
set -eu

wasm_target="${1:-wasm32-unknown-unknown}"

if ! rustup target list --installed 2>/dev/null | grep -F -x -q "$wasm_target"; then
    echo "wasm Firefox dispatch: skipping $wasm_target; Rust target is not installed"
    exit 0
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "wasm Firefox dispatch: skipping Firefox smoke; python3 is not installed"
    exit 0
fi

driver="${BASE64_NG_GECKODRIVER:-}"
if [ -z "$driver" ]; then
    if command -v geckodriver >/dev/null 2>&1; then
        driver="$(command -v geckodriver)"
    elif [ -x "$HOME/.cargo/bin/geckodriver" ]; then
        driver="$HOME/.cargo/bin/geckodriver"
    else
        echo "wasm Firefox dispatch: skipping Firefox smoke; set BASE64_NG_GECKODRIVER=/path/to/geckodriver"
        exit 0
    fi
fi

python3 scripts/wasm_webdriver_smoke.py \
    --browser firefox \
    --driver "$driver" \
    --target "$wasm_target"
