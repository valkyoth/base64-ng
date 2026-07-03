# base64-ng 1.1.0 Release Notes

Status: released

## Summary

- Started the SIMD encode foundation line with a real SSSE3/SSE4.1 fixed-block
  encode prototype for Standard and URL-safe alphabets. The prototype remains
  non-dispatchable; active runtime backend selection is still scalar-only.
- Replaced the previous SSSE3/SSE4.1 zero-output scaffold with SSSE3 byte
  shuffling, SSE lane shifts/masks, and SSE4.1 byte blending for 12-byte input
  blocks encoded to 16 Base64 bytes.
- Added deterministic SIMD equivalence coverage that exercises patterned input
  blocks and all 64 emitted six-bit Base64 values against the scalar encoder.
- Added explicit test-prototype XMM register cleanup and updated the unsafe
  inventory for the new vectorized SSSE3/SSE4.1 encode path.
- Hardened the SSSE3/SSE4.1 prototype by wiping its staged stack copy before
  return and requiring a complete Standard-family alphabet match before the
  vectorized Standard/URL-safe mapper is used.

## Commit Range

- Previous tag: `v1.0.10`
- Release tag: `v1.1.0`
- Release date: `2026-06-20`

## Commits

### Security / Hardening

- `7389dfe` Harden SSSE3 encode prototype

### Documentation

- `3158e4e` Plan encode and decode SIMD release lines
- `0300407` Clarify 1.1 SIMD release posture

### Verification

- `87291e9` Gate x86 SIMD test helpers

### Other Changes

- `7bb2a4a` Fix dependent crate publish dry-runs
- `c5d2cc5` Refine post-1.0 SIMD roadmap
- `0d31e09` Start 1.1.0 SIMD encode foundation
- `b046d2f` Update harness locks for 1.1.0

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.1.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
