# base64-ng 0.3.0 Release Notes

Status: released

## Summary

- Started the next development cycle after the `0.2.0` release.
- Added an initial `ct` scalar decode module for caller-owned buffers. The path
  avoids secret-indexed lookup tables during Base64 symbol mapping while
  remaining explicit that it is not yet a formally verified cryptographic
  constant-time API.
- Extended fuzz coverage to check `ct` decode success/error parity against the
  strict scalar decoder.
- Extended exhaustive short canonical round-trip tests to cover the `ct`
  decoder for all one- and two-byte inputs across all built-in engines.
- Changed `ct` malformed-input reporting to non-localized sentinel errors so
  error tracking does not expose the first malformed byte position.
- Changed `ct` padding-length calculation to use mask arithmetic instead of
  branch-based equality checks.
- Added a SIMD admission policy that keeps hardware acceleration out of `0.3`
  until unsafe code isolation, dispatch behavior, fuzzing, CI, and benchmark
  evidence are ready.
- Added an isolated no-framework scalar performance comparison harness against
  the established `base64` crate.
- Added performance harness dependency audit and license checks to CI and the
  release gate.
- Documented how performance evidence should be generated and qualified for
  releases.
- Documented scalar decode throughput as an explicit optimization target rather
  than a release claim.
- Added strict and legacy in-place decode variants that clear unused buffer
  bytes on success and clear the full caller buffer on error.
- Extended in-place fuzz coverage to verify clear-tail decode result parity and
  cleanup semantics.
- Documented clear-tail decode cleanup as best-effort buffer-retention
  reduction rather than a formal zeroization guarantee.
- Added an in-place encode variant that clears unused buffer bytes on success
  and clears the full caller buffer on error.
- Extended in-place fuzz coverage to verify clear-tail encode result parity and
  cleanup semantics.
- Added slice-output encode and decode variants that clear unused output bytes
  on success and clear the output buffer on error.
- Extended decode fuzz coverage to verify clear-tail slice encode/decode result
  parity and cleanup semantics.
- Added rustdoc examples for clear-tail slice APIs so the cleanup contract is
  visible in generated API documentation.
- Updated the roadmap to reflect the dependency-free clear-tail design instead
  of adding a default cleanup dependency.

## Commit Range

- Previous tag: `v0.2.0`
- Release tag: `v0.3.0`
- Release date: `2026-05-13`

## Commits

### Added

- `4b3a183` Add isolated scalar performance harness
- `b2423bf` Add clear-tail in-place decode APIs
- `c0de79d` Add clear-tail in-place encode API
- `ada8923` Add clear-tail slice APIs
- `f7a5675` Add constant-time oriented decode API
- `813b185` Expand constant-time decode coverage
- `490dee8` Use branchless ct padding length

### Documentation

- `d052304` Document scalar decode performance target
- `190ad64` Document clear-tail cleanup limits
- `066daf8` Document clear-tail slice APIs
- `67ea8bb` Document SIMD admission policy
- `6e01c89` Prepare 0.3.0 release

### Verification

- `c839d29` Fuzz clear-tail in-place decode
- `fd1d21a` Fuzz clear-tail in-place encode
- `afef25e` Fuzz clear-tail slice APIs

### Other Changes

- `fea56f1` Start 0.3.0 development cycle
- `a502f1b` Align roadmap with clear-tail design
- `6a2ab91` Remove localized error tracking from ct decode

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.3.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
