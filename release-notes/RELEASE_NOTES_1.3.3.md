# base64-ng 1.3.3

This patch keeps the workspace crate family synchronized and admits a narrow
wasm `simd128` runtime-dispatch profile with explicit runtime smoke evidence.

## Changed

- Admitted wasm `simd128` runtime dispatch for Standard and URL-safe public
  encode plus normal strict decode when `wasm32` binaries are compiled with
  `target-feature=+simd128`, `simd`, and
  `allow-wasm32-best-effort-wipe`.
- Added Node/V8 and Wasmtime runtime smoke evidence requiring `wasm-simd128`
  candidate, active encode, and active decode backend reporting plus Standard
  and URL-safe deterministic length sweeps, independent scalar reference
  encode checks, malformed-input rejection, and public API round trips.
- Added Chromium-family browser runtime smoke evidence for the same admitted
  wasm `simd128` profile.
- Added Firefox/SpiderMonkey WebDriver runtime smoke evidence through
  `geckodriver` for the same admitted wasm `simd128` profile.
- Added Safari/WebKit WebDriver runtime smoke evidence through `safaridriver`
  for the same admitted wasm `simd128` profile.
- Added fail-closed scalar verification for wasm fixed-block encode before
  copying staged SIMD output to caller output.
- Hardened wasm scalar-verification cleanup so staged stack buffers are wiped
  before any verification error returns, and kept the unsafe block scoped to
  the fixed-size wasm SIMD view/call.
- Added strict decode error-surface evidence for Standard and URL-safe
  padded/unpadded public helpers, including clear-tail wiping on rejected
  input.
- Routed strict wrapped decode through the admitted strict decode backend after
  scalar line-profile validation and fixed-size line-ending compaction. Line
  handling remains scalar; the compacted strict Base64 chunks may use admitted
  Standard/URL-safe decode acceleration.
- Routed legacy whitespace decode through the admitted strict decode backend
  after scalar whitespace validation and fixed-size compaction. Whitespace
  handling and public error positions remain scalar-governed.
- Added release-gated wasm SIMD codegen evidence through
  `scripts/generate_wasm_simd_evidence.sh`, which emits test-harness LLVM IR
  with `target-feature=+simd128` when the wasm target is installed.
- Added `Engine::profile_with_wrap` and `Engine::checked_profile_with_wrap` for
  simpler construction of strict wrapped profiles.
- Updated README, release evidence, SIMD admission, and roadmap documentation
  to describe the narrow admitted wasm profile and the remaining browser/JIT,
  zeroization, and benchmark caveats.

## Security Notes

- Wasm `simd128` is admitted only for the named Node/V8, Wasmtime,
  Chromium-family browser, Firefox/SpiderMonkey, and Safari/WebKit smoke
  profiles and the narrow Standard/URL-safe encode plus strict-decode surface.
- Browser-wide behavior beyond the named Chromium-family, Firefox/SpiderMonkey,
  and Safari/WebKit runtimes,
  runtime/JIT timing behavior, and performance claims remain out of scope
  until separately evidenced.
- The default wasm cleanup policy remains fail-closed unless
  `allow-wasm32-best-effort-wipe` is explicitly enabled.
