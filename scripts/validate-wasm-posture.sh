#!/usr/bin/env sh
set -eu

simd_mod="src/simd/mod.rs"
runtime_report="src/runtime/report.rs"
simd_doc="docs/SIMD.md"
admission_doc="docs/SIMD_ADMISSION.md"
review_doc="docs/WASM_SIMD128_RUNTIME_REVIEW.md"
wipe_script="scripts/check_wasm_wipe_policy.sh"
feature_script="scripts/check_simd_feature_bundles.sh"

require_text() {
    file="$1"
    text="$2"
    if ! grep -F -q "$text" "$file"; then
        echo "wasm posture: $file is missing required text: $text" >&2
        exit 1
    fi
}

echo "wasm posture: checking non-dispatchable simd128 policy"

require_text "$simd_doc" "Runtime backend selection remains scalar for wasm."
require_text "$simd_doc" "WASM_SIMD128_RUNTIME_REVIEW.md"
require_text "$admission_doc" "wasm \`simd128\` remains compile/codegen evidence only"
require_text "$admission_doc" "Candidate reporting may expose \`wasm-simd128\`, but active encode and"
require_text "$admission_doc" "decode backends remain scalar on wasm32."
require_text "$admission_doc" "test-binary compile evidence only; non-dispatchable"
require_text "$admission_doc" "WASM_SIMD128_RUNTIME_REVIEW.md"
require_text "$review_doc" "No wasm \`simd128\` runtime dispatch is admitted in \`1.3.3\`."
require_text "$review_doc" "Candidate reporting may expose \`wasm-simd128\`"
require_text "$review_doc" "active encode and decode backends remain scalar on \`wasm32\`"
require_text "$review_doc" "src/simd/mod.rs\` must not include \`WasmSimd128\` in \`ActiveBackend\`"
require_text "$wipe_script" "allow-wasm32-best-effort-wipe"
require_text "$feature_script" "target-feature=+simd128"
require_text "$runtime_report" "Candidate::WasmSimd128 => Backend::WasmSimd128"

if awk '
    /enum ActiveBackend/ { inside = 1 }
    inside && /}/ { inside = 0 }
    inside && /WasmSimd128/ { found = 1 }
    END { exit found ? 0 : 1 }
' "$simd_mod"; then
    echo "wasm posture: ActiveBackend must not include WasmSimd128" >&2
    exit 1
fi

require_text "$simd_mod" "WasmSimd128"
require_text "$simd_mod" "fixed-block implementation remains prototype evidence and is not reachable"
require_text "$simd_mod" "from runtime backend selection."

echo "wasm posture: ok"
