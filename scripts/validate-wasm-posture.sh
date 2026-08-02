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
loader_script="scripts/check-2.0-wasm-loader.sh"
loader_browser_script="scripts/check_wasm_loader_browser_dispatch.sh"
loader_firefox_script="scripts/check_wasm_loader_browser_firefox_dispatch.sh"
loader_safari_script="scripts/check_wasm_loader_browser_safari_dispatch.sh"
loader_source="packages/base64-ng-wasm-loader/src/index.js"

echo "wasm posture: checking admitted simd128 runtime policy"

require_text "$simd_doc" "supported \`base64-ng-wasm-loader\` npm package"
require_text "$simd_doc" "Exact-package Node/V8, Wasmtime, Chromium/V8,"
require_text "$simd_doc" "Firefox/SpiderMonkey"
require_text "$simd_doc" "Safari/WebKit"
require_text "$simd_doc" "WASM_SIMD128_RUNTIME_REVIEW.md"
require_text "$admission_doc" "wasm \`simd128\` is admitted as direct fixed-block encode and"
require_text "$admission_doc" "\`base64-ng-wasm-loader\` package ships separately selected scalar and SIMD"
require_text "$admission_doc" "Wasmtime, Chromium/V8, Firefox/SpiderMonkey, and operator-run Safari/WebKit"
require_text "$admission_doc" "| wasm \`simd128\` | admitted backend |"
require_text "$admission_doc" "WASM_SIMD128_RUNTIME_REVIEW.md"
require_text "$review_doc" "2.0 Commit 30"
require_text "$review_doc" "Node/V8"
require_text "$review_doc" "Wasmtime"
require_text "$review_doc" "Chromium/V8"
require_text "$review_doc" "Firefox/SpiderMonkey"
require_text "$review_doc" "Safari/WebKit"
require_text "$review_doc" "ships two immutable artifacts"
require_text "$review_doc" "whole-input scalar validation"
require_text "$review_doc" "closed intrinsic allowlist"
require_text "$review_doc" "embedded SHA-256 digest"
require_text "$review_doc" "complete scratch clear on \`dispose()\`"
require_text "$review_doc" "no \`eval\` or \`new Function\`"
require_text "$review_doc" "script-src 'wasm-unsafe-eval'"
require_text "$review_doc" "scripts/generate_wasm_simd_evidence.sh"
require_text "$review_doc" "scripts/check_wasm_runtime_dispatch.sh"
require_text "$review_doc" "scripts/check-2.0-wasm-loader.sh"
require_text "$review_doc" "scripts/check_wasm_loader_browser_dispatch.sh"
require_text "$review_doc" "scripts/check_wasm_loader_browser_firefox_dispatch.sh"
require_text "$review_doc" "scripts/check_wasm_loader_browser_safari_dispatch.sh"
require_text "$wipe_script" "allow-wasm32-best-effort-wipe"
require_text "$feature_script" "target-feature=+simd128"
require_text "$evidence_script" "target-feature=+simd128"
require_text "$evidence_script" "wasm bitselect intrinsic"
require_text "$evidence_script" "whole-vector validity reduction"
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
require_text "$loader_script" "deterministic scalar and simd128 artifacts"
require_text "$loader_script" "exact npm package and install smoke"
require_text "$loader_browser_script" "wasm_loader_browser_smoke.py"
require_text "$loader_firefox_script" "geckodriver"
require_text "$loader_safari_script" "safaridriver"
require_text "$loader_source" "WebAssembly.validate"
require_text "$loader_source" "runtime-intrinsics-unavailable"
require_text "$loader_source" "overlapping-views"
require_text "$loader_source" "base64_ng_clear_used"
require_text "$loader_source" "artifact-integrity-policy"
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
