# base64-ng 1.3.0 Release Notes

Status: released

## Summary

- Admitted normal strict SIMD decode for Standard and URL-safe alphabet
  families on std `x86`/`x86_64` AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and
  little-endian std `aarch64` NEON after whole-input scalar validation.
- Kept wrapped decode, legacy whitespace decode, custom alphabets,
  bcrypt-style and `crypt(3)` profiles, in-place decode, `no_std`, wasm
  runtime dispatch, and constant-time-oriented secret decode scalar unless a
  future evidence package admits them separately.
- Added scalar-equivalence, malformed-input, dispatch-boundary, fuzz,
  benchmark, hardware-check, unsafe-inventory, and backend-evidence coverage
  for the admitted strict decode scope.
- Added `base64-ng-tokio` manual `AsyncRead` streaming adapters with fixed
  buffers, cancellation/resume tests, chunk-boundary tests, drop cleanup, and
  bounded helper APIs. Async writer adapters remain deferred.
- Added strict const decode helpers for fixed static inputs with recoverable
  `Result` errors instead of new runtime panic surfaces.
- Expanded companion-crate coverage for serde profile modules, bytes helpers,
  subtle comparison examples, sanitization locked-secret direct-fill guidance,
  derive examples, and Tokio streaming documentation.
- Expanded bounded Kani verification for the decode backend/scalar agreement
  path and added an opt-in advanced Kani script for expensive background
  harnesses.
- Refreshed benchmark evidence, backend evidence, and wasm posture
  documentation. Wasm `simd128` remains compile/codegen evidence only and is
  not admitted for runtime dispatch.

## Commit Range

- Previous tag: `v1.2.3`
- Release tag: `v1.3.0`
- Release date: `2026-07-03`

## Commits

### Added

- `4fad417` Add Rust 1.96.1 compatibility coverage
- `f4a7b7c` Admit SSSE3 strict decode dispatch
- `8aace53` Admit AVX2 strict decode dispatch
- `6111e83` Admit AVX-512 strict decode dispatch
- `b9a9f9c` Admit AArch64 NEON strict decode dispatch
- `7e823f1` Add limited Tokio read-all helpers
- `510e308` Add const decode array API
- `b6e2e9e` Expand serde field profile modules

### Security / Hardening

- `9a4ccdd` Address decode scope pentest follow-up
- `0107188` Harden strict decode backend boundary tests
- `acdf182` Harden decode prototype parity checks
- `6f737f1` Harden SIMD decode unsafe inventory
- `68e9bee` Harden Tokio helper cleanup
- `f2117f3` Harden Tokio reader cleanup paths
- `9f02eb9` Address low severity pentest hardening

### Documentation

- `d70e98d` Document 1.3.0 completion commit plan
- `2e929f9` Add Tokio async reader streaming adapters
- `47ee74c` Document wasm SIMD posture
- `ce8c816` Sync 1.3.0 release candidate docs
- `f128f30` Refresh README examples for 1.3.0
- `2ab2222` Prepare 1.3.0 release candidate
- `3193baf` Plan post-1.3 expansion slices

### Verification

- `8a77fae` Add SSSE3 decode prototype evidence
- `8897576` Add AVX2 decode prototype evidence
- `b52dfd2` Add AVX-512 decode prototype evidence
- `14735a8` Add NEON decode prototype evidence
- `e4db304` Expand decode malformed-input evidence
- `8050dcd` Expand bounded Kani decode backend proof
- `6998e02` Add advanced Kani proof gate
- `756f18b` Refresh benchmark backend evidence
- `b12ccb7` Extend CI Miri timeout

### Other Changes

- `170024d` Freeze SIMD decode admission scope
- `3355b6e` Constrain NEON admission to little-endian AArch64
- `ca4053d` Review remaining encode acceleration surfaces

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.3.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
