# base64-ng 1.1.1 Release Notes

Status: released

## Summary

`1.1.1` contains the commits listed below. No dedicated
changelog section was present when this historical release-note
file was generated, so this summary is reconstructed from git
commit subjects.

## Commit Range

- Previous tag: `v1.1.0`
- Release tag: `v1.1.1`
- Release date: `2026-06-20`

## Commits

### Added

- `317f72f` Add AVX2 encode prototype
- `b6b882c` Add AVX-512 encode prototype
- `de2d5fa` Add AArch64 NEON encode prototype
- `0f7ee2a` Add wasm simd128 encode prototype
- `520ee23` Add SIMD encode admission draft

### Security / Hardening

- `b828f2b` Harden SIMD admission evidence labels
- `a3f8a2a` Harden cleanup and SIMD admission docs

### Verification

- `28b8f2a` Add SIMD assembly evidence capture
- `514cd50` Remove wasm SIMD test unwraps

### Other Changes

- `f402871` Fix SIMD prototype block staging
- `6b98c78` Fix NEON doc markdown

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.1.1.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
