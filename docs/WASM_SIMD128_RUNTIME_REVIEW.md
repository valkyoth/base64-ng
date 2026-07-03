# Wasm `simd128` Runtime Review

This file tracks the `1.3.3` wasm runtime-dispatch decision and admission
evidence.

## Decision

wasm `simd128` runtime dispatch is admitted in `1.3.3` for binaries compiled
with `target-feature=+simd128`, the `simd` feature, and the explicit
`allow-wasm32-best-effort-wipe` feature.

The admitted runtime profile is intentionally narrow:

- Standard and URL-safe alphabet families only
- normal encode through the public encode boundary
- in-place encode only through stack staging before the public encode boundary
- normal strict decode through the public strict decode boundary
- strict in-place decode only through stack staging before the public strict
  decode boundary
- strict wrapped decode may enter the public strict decode boundary after
  scalar line-profile validation and line-ending compaction
- legacy whitespace decode may enter the public strict decode boundary after
  scalar whitespace compaction
- full fixed blocks use `simd128`; tails and unsupported surfaces use scalar
- no vectorized line-profile validation, line-ending compaction, or
  legacy-whitespace compaction; no custom alphabets, bcrypt-style profiles,
  or `ct` secret decode
- runtime smoke evidence is required for Node/V8, Wasmtime, at least one
  Chromium-family browser, Firefox/SpiderMonkey, and Safari/WebKit

When compiled with `simd128`, active encode and decode backends are `wasm-simd128`
on `wasm32`.

## Evidence

The admission is backed by:

- `scripts/check_wasm_runtime_dispatch.sh`, which builds a `cdylib` smoke
  module with `RUSTFLAGS='-C target-feature=+simd128'` and executes it under
  Node/V8 and Wasmtime.
- `scripts/check_wasm_browser_dispatch.sh`, which executes the same smoke
  module in a Chromium-family browser through synchronous browser
  `WebAssembly.Module` instantiation.
- `scripts/check_wasm_browser_firefox_dispatch.sh`, which executes the same
  smoke module in Firefox/SpiderMonkey through `geckodriver`.
- `scripts/check_wasm_browser_safari_dispatch.sh`, which executes the same
  smoke module in Safari/WebKit through `safaridriver` when Safari remote
  automation is enabled on macOS.
- The smoke checks `runtime::backend_report()` and requires candidate, active
  encode, and active decode backend reporting to be `wasm-simd128`.
- The smoke runs a deterministic length sweep from 0 through 200 bytes with
  multiple seeds for Standard padded and URL-safe no-padding payloads.
- The smoke compares public encode output against an independent scalar reference encoder
  before decoding it back through the public strict decode APIs.
- The smoke includes malformed block-boundary inputs that must return decode
  errors without trapping.
- `scripts/generate_wasm_simd_evidence.sh`, which emits release test-harness
  LLVM IR and checks for `simd128` codegen markers.
- `scripts/check_simd_feature_bundles.sh`, which keeps compile/test-binary
  evidence for `wasm32-unknown-unknown` with `target-feature=+simd128` when
  the target is installed.

## Limits

Wasm execution still includes a runtime or JIT layer outside the Rust compiler
and outside this crate's control. This admission proves correctness and
dispatch behavior for the named runtime smoke profile; it does not claim:

- browser-wide behavior beyond the named Chromium-family, Firefox/SpiderMonkey,
  and Safari/WebKit smoke runtimes
- runtime/JIT timing behavior for every V8, SpiderMonkey, Wasmtime, Wasmer, or
  edge-compute deployment
- hardware register-retention cleanup guarantees after the wasm runtime lowers
  wasm to native code
- stronger zeroization behavior than the documented wasm wipe-barrier caveat
- performance superiority without local benchmark evidence

## Required Before Admission

Before broadening wasm `simd128` beyond this admitted profile, a release must
provide:

- a named additional wasm runtime profile, not a generic "wasm" claim
- scalar differential tests for encode, clear-tail, allocation helpers, and
  every admitted decode surface if decode is included
- generated wasm/codegen evidence with `target-feature=+simd128`
- runtime tests in the admitted engine profile
- fallback tests proving scalar execution when `simd128` is unavailable
- benchmark evidence that names the wasm engine, version, host CPU, operating
  system, Rust version, and exact commands
- explicit documentation of the wasm wipe-barrier limitation and the
  `allow-wasm32-best-effort-wipe` opt-in
- release notes that distinguish candidate detection from active dispatch

## Current Enforcement

- `src/simd/mod.rs` includes `WasmSimd128` in `ActiveBackend` only behind
  `cfg(all(feature = "simd", target_arch = "wasm32"))`.
- `src/simd/wasm.rs` stages wasm encode output, compares it against scalar
  output before copying bytes to the caller's output buffer, and wipes staged
  stack buffers on every verification failure path before returning.
  Evidence phrase: compares it against scalar output before copying bytes to the caller's output buffer.
  Evidence phrase: wipes staged stack buffers on every verification failure path before returning.
- `scripts/validate-wasm-posture.sh` checks this review document, the SIMD
  documentation, the admission manifest, and the runtime boundary.
- `scripts/check_wasm_runtime_dispatch.sh` executes the runtime smoke under
  Node/V8 and Wasmtime when the tools are installed.
- `scripts/check_wasm_browser_dispatch.sh` executes the same runtime smoke in
  a Chromium-family browser when Chrome/Chromium is installed or
  `BASE64_NG_BROWSER` points to a compatible browser binary.
- `scripts/check_wasm_browser_firefox_dispatch.sh` executes the same runtime
  smoke through `geckodriver` when Firefox evidence is requested locally or in
  CI.
- `scripts/check_wasm_browser_safari_dispatch.sh` executes the same runtime
  smoke through `safaridriver` when Safari remote automation is enabled on
  macOS. The `1.3.3` release evidence includes a macOS Safari pass reported
  from `/System/Cryptexes/App/usr/bin/safaridriver`.
- `scripts/check_simd_feature_bundles.sh` keeps compile/test-binary evidence
  for `wasm32-unknown-unknown` with `target-feature=+simd128` when the target
  is installed.
- `scripts/generate_wasm_simd_evidence.sh` emits release test-harness LLVM IR
  and checks for `simd128` codegen markers.
- `scripts/check_wasm_wipe_policy.sh` keeps the wasm cleanup posture
  fail-closed by default unless `allow-wasm32-best-effort-wipe` is explicitly
  enabled.
