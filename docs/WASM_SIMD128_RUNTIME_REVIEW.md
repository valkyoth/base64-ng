# Wasm `simd128` Runtime Review

This file tracks the `1.3.3` wasm runtime-dispatch decision. It is an
admission review, not an acceleration announcement.

## Decision

No wasm `simd128` runtime dispatch is admitted in `1.3.3`.

The existing wasm `simd128` code remains real fixed-block encode prototype
evidence for Standard and URL-safe alphabet families. It is compiled and
typechecked when the wasm target and `target-feature=+simd128` are available,
but it is not reachable from public encode or decode APIs.

Candidate reporting may expose `wasm-simd128` when the binary is compiled with
`simd128`, but active encode and decode backends remain scalar on `wasm32`.

## Rationale

Wasm execution includes a runtime or JIT layer outside the Rust compiler and
outside this crate's control. The current project evidence proves that the
prototype code compiles and matches scalar fixed-block output, but it does not
prove the properties needed for active dispatch:

- runtime/JIT timing behavior for V8, SpiderMonkey, Wasmtime, Wasmer, and
  other deployment engines
- register-retention or value-retention posture in the wasm runtime
- memory cleanup behavior after the Rust-generated wasm is optimized by a
  downstream runtime
- production fallback behavior across engines with and without `simd128`
- benchmark evidence from named engines and deployment profiles

Because that evidence is incomplete, the secure decision is to keep wasm
runtime execution scalar and keep the vector code as compile/codegen evidence
only.

## Required Before Admission

Before wasm `simd128` can become an active backend, a release must provide:

- a named admitted wasm runtime profile, not a generic "wasm" claim
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

- `src/simd/mod.rs` must not include `WasmSimd128` in `ActiveBackend`.
- `scripts/validate-wasm-posture.sh` checks this review document, the SIMD
  documentation, the admission manifest, and the runtime boundary.
- `scripts/check_simd_feature_bundles.sh` keeps compile/test-binary evidence
  for `wasm32-unknown-unknown` with `target-feature=+simd128` when the target
  is installed.
- `scripts/generate_wasm_simd_evidence.sh` emits release test-harness LLVM IR
  for the inactive prototype and checks for `simd128` codegen markers. This is
  compile/codegen evidence only; it is not runtime/JIT admission evidence.
- `scripts/check_wasm_wipe_policy.sh` keeps the wasm cleanup posture
  fail-closed by default unless `allow-wasm32-best-effort-wipe` is explicitly
  enabled.
