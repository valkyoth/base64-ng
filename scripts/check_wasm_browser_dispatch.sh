#!/usr/bin/env sh
set -eu

wasm_target="${1:-wasm32-unknown-unknown}"
smoke_dir="target/wasm-runtime-smoke"
wasm_file="$smoke_dir/target/$wasm_target/release/base64_ng_wasm_runtime_smoke.wasm"
html_file="$smoke_dir/browser-smoke.html"
browser_output="$smoke_dir/browser-smoke-output.html"
success_marker='data-base64-ng-wasm-smoke="pass"'
browser_profile=""

cleanup() {
    if [ -n "$browser_profile" ] && [ -d "$browser_profile" ]; then
        rm -rf "$browser_profile"
    fi
}
trap cleanup EXIT HUP INT TERM

find_browser() {
    if [ -n "${BASE64_NG_BROWSER:-}" ]; then
        printf '%s\n' "$BASE64_NG_BROWSER"
        return
    fi

    for candidate in google-chrome chromium chromium-browser chrome microsoft-edge; do
        if command -v "$candidate" >/dev/null 2>&1; then
            command -v "$candidate"
            return
        fi
    done

    for candidate in \
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
        "/Applications/Chromium.app/Contents/MacOS/Chromium" \
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
    do
        if [ -x "$candidate" ]; then
            printf '%s\n' "$candidate"
            return
        fi
    done

    printf '%s\n' ""
}

if ! rustup target list --installed 2>/dev/null | grep -F -x -q "$wasm_target"; then
    echo "wasm browser dispatch: skipping $wasm_target; Rust target is not installed"
    exit 0
fi

if [ ! -s "$wasm_file" ]; then
    echo "wasm browser dispatch: smoke module missing; building through runtime smoke"
    scripts/check_wasm_runtime_dispatch.sh "$wasm_target"
fi

browser_bin="$(find_browser)"
if [ -z "$browser_bin" ]; then
    echo "wasm browser dispatch: skipping browser smoke; set BASE64_NG_BROWSER=/path/to/chrome-or-chromium"
    exit 0
fi

wasm_base64="$(base64 <"$wasm_file" | tr -d '\n')"
forgiving_fixtures="$(python3 scripts/whatwg_forgiving_fixtures.py)"

cat >"$html_file" <<EOF
<!doctype html>
<html>
<head><meta charset="utf-8"><title>base64-ng wasm browser smoke</title></head>
<body data-base64-ng-wasm-smoke="pending">
<pre id="result">pending</pre>
<script>
const wasmBase64 = "$wasm_base64";
const forgivingFixtures = $forgiving_fixtures;

function hexToString(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return new TextDecoder("utf-8", {fatal: true}).decode(bytes);
}

function bytesToHex(input) {
  let output = "";
  for (let index = 0; index < input.length; index += 1) {
    output += input.charCodeAt(index).toString(16).padStart(2, "0");
  }
  return output;
}

function verifyForgivingFixtures() {
  for (const [inputHex, expected] of forgivingFixtures) {
    let actual = null;
    let failed = false;
    try {
      actual = bytesToHex(atob(hexToString(inputHex)));
    } catch (_) {
      failed = true;
    }
    if ((expected === null) !== failed || (!failed && actual !== expected)) {
      throw new Error("Chromium forgiving fixture mismatch: " + inputHex);
    }
  }
}

function decodeBase64(input) {
  const binary = atob(input);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

try {
  verifyForgivingFixtures();
  const module = new WebAssembly.Module(decodeBase64(wasmBase64));
  const instance = new WebAssembly.Instance(module, {});
  const result = instance.exports.base64_ng_wasm_runtime_smoke();
  if (result === 0) {
    document.body.setAttribute("data-base64-ng-wasm-smoke", "pass");
    document.getElementById("result").textContent = "BASE64_NG_BROWSER_WASM_SMOKE_PASS";
  } else {
    document.body.setAttribute("data-base64-ng-wasm-smoke", "fail-" + result);
    document.getElementById("result").textContent = "BASE64_NG_BROWSER_WASM_SMOKE_FAIL_" + result;
  }
} catch (error) {
  document.body.setAttribute("data-base64-ng-wasm-smoke", "exception");
  document.getElementById("result").textContent = "BASE64_NG_BROWSER_WASM_SMOKE_EXCEPTION " + error;
}
</script>
</body>
</html>
EOF

if grep -F -q "$success_marker" "$html_file"; then
    echo "wasm browser dispatch: success marker must not exist in static HTML" >&2
    exit 1
fi

case "$html_file" in
    /*) html_url="file://$html_file" ;;
    *) html_url="file://$(pwd)/$html_file" ;;
esac

echo "wasm browser dispatch: running Chromium-family browser smoke with $browser_bin"
browser_profile="$(mktemp -d "$smoke_dir/chromium-profile.XXXXXX")"
"$browser_bin" \
    --headless=new \
    --disable-gpu \
    --no-sandbox \
    --disable-dev-shm-usage \
    --user-data-dir="$browser_profile" \
    --dump-dom \
    "$html_url" >"$browser_output" 2>&1

if ! grep -F -q "$success_marker" "$browser_output"; then
    echo "wasm browser dispatch: browser smoke failed" >&2
    cat "$browser_output" >&2
    exit 1
fi

echo "wasm browser dispatch: ok"
