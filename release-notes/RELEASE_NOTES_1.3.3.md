# base64-ng 1.3.3

This patch keeps the workspace crate family synchronized and tightens the wasm
SIMD release posture without admitting new wasm runtime acceleration.

## Changed

- Recorded that wasm `simd128` remains compile/codegen evidence only in
  `1.3.3`; active encode and decode dispatch remain scalar on `wasm32`.
- Added release-gated wasm SIMD codegen evidence through
  `scripts/generate_wasm_simd_evidence.sh`, which emits test-harness LLVM IR
  with `target-feature=+simd128` when the wasm target is installed.
- Added `Engine::profile_with_wrap` and `Engine::checked_profile_with_wrap` for
  simpler construction of strict wrapped profiles.
- Updated README, release evidence, SIMD admission, and roadmap documentation
  to distinguish wasm candidate reporting from active runtime dispatch.

## Security Notes

- No new SIMD runtime backend is admitted in this release.
- Wasm `simd128` still requires named runtime/JIT evidence, fallback evidence,
  benchmark evidence, and cleanup-posture review before it can become active.
- The default wasm cleanup policy remains fail-closed unless
  `allow-wasm32-best-effort-wipe` is explicitly enabled.
