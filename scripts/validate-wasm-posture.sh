#!/usr/bin/env sh
set -eu

simd_mod="src/simd/mod.rs"
runtime_report="src/runtime/report.rs"
simd_doc="docs/SIMD.md"
admission_doc="docs/SIMD_ADMISSION.md"
review_doc="docs/WASM_SIMD128_RUNTIME_REVIEW.md"
wipe_script="scripts/check_wasm_wipe_policy.sh"
feature_script="scripts/check_simd_feature_bundles.sh"
evidence_script="scripts/generate_wasm_simd_evidence.sh"

require_text() {
    file="$1"
    text="$2"
    if ! grep -F -q "$text" "$file"; then
        echo "wasm posture: $file is missing required text: $text" >&2
        exit 1
    fi
}

runtime_script="scripts/check_wasm_runtime_dispatch.sh"
browser_script="scripts/check_wasm_browser_dispatch.sh"
firefox_script="scripts/check_wasm_browser_firefox_dispatch.sh"
safari_script="scripts/check_wasm_browser_safari_dispatch.sh"

echo "wasm posture: checking admitted simd128 runtime policy"

require_text "$simd_doc" "Node/V8, Wasmtime, Chromium-family browser,"
require_text "$simd_doc" "Firefox/SpiderMonkey, and Safari/WebKit runtime smoke evidence"
require_text "$simd_doc" "Chromium-family browser"
require_text "$simd_doc" "Firefox/SpiderMonkey"
require_text "$simd_doc" "Safari/WebKit"
require_text "$simd_doc" "WASM_SIMD128_RUNTIME_REVIEW.md"
require_text "$admission_doc" "wasm \`simd128\` is admitted for runtime dispatch"
require_text "$admission_doc" "Node/V8, Wasmtime, Chromium-family"
require_text "$admission_doc" "browser, Firefox/SpiderMonkey, and Safari/WebKit runtime smoke evidence"
require_text "$admission_doc" "Firefox/SpiderMonkey runtime smoke evidence"
require_text "$admission_doc" "Safari/WebKit runtime smoke evidence"
require_text "$admission_doc" "| wasm \`simd128\` | admitted backend |"
require_text "$admission_doc" "WASM_SIMD128_RUNTIME_REVIEW.md"
require_text "$review_doc" "wasm \`simd128\` runtime dispatch is admitted in \`1.3.3\`"
require_text "$review_doc" "Node/V8"
require_text "$review_doc" "Wasmtime"
require_text "$review_doc" "Chromium-family browser"
require_text "$review_doc" "Firefox/SpiderMonkey"
require_text "$review_doc" "Safari/WebKit"
require_text "$review_doc" "active encode and decode backends are \`wasm-simd128\`"
require_text "$review_doc" "deterministic length sweep from 0 through 200 bytes"
require_text "$review_doc" "independent scalar reference encoder"
require_text "$review_doc" "malformed block-boundary inputs"
require_text "$review_doc" "compares it against scalar output before copying bytes to the caller's output buffer"
require_text "$review_doc" "wipes staged stack buffers on every verification failure path before returning"
require_text "$review_doc" "scripts/generate_wasm_simd_evidence.sh"
require_text "$review_doc" "scripts/check_wasm_runtime_dispatch.sh"
require_text "$review_doc" "scripts/check_wasm_browser_dispatch.sh"
require_text "$review_doc" "scripts/check_wasm_browser_firefox_dispatch.sh"
require_text "$review_doc" "scripts/check_wasm_browser_safari_dispatch.sh"
require_text "$wipe_script" "allow-wasm32-best-effort-wipe"
require_text "$feature_script" "target-feature=+simd128"
require_text "$evidence_script" "target-feature=+simd128"
require_text "$evidence_script" "wasm bitselect intrinsic"
require_text "$evidence_script" "does not attest any runtime/JIT timing or cleanup behavior"
require_text "$runtime_script" "Node/V8"
require_text "$runtime_script" "Wasmtime"
require_text "$runtime_script" "0..=MAX_INPUT"
require_text "$runtime_script" "reference_encode"
require_text "$runtime_script" "check_rejects_malformed"
require_text "$browser_script" "Chromium-family browser"
require_text "$browser_script" "WebAssembly.Module"
require_text "$browser_script" "success marker must not exist in static HTML"
require_text "$browser_script" "data-base64-ng-wasm-smoke=\"pass\""
require_text "$firefox_script" "geckodriver"
require_text "$safari_script" "safaridriver"
require_text "$runtime_report" "Candidate::WasmSimd128 => Backend::WasmSimd128"

if ! awk '
    /enum ActiveBackend/ { inside = 1 }
    inside && /}/ { inside = 0 }
    inside && /WasmSimd128/ { found = 1 }
    END { exit found ? 0 : 1 }
' "$simd_mod"; then
    echo "wasm posture: ActiveBackend must include admitted WasmSimd128" >&2
    exit 1
fi

require_text "$simd_mod" "WasmSimd128"
require_text "$simd_mod" "wasm \`simd128\`"

echo "wasm posture: ok"
