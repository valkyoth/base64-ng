# base64-ng 1.0.1 Release Notes

Status: released

## Summary

- Lowered the documented MSRV from Rust `1.95.0` to Rust `1.90.0` after
  confirming the current code builds, tests, lints, and documents cleanly on
  Rust `1.90.0` without code changes.
- Added README compiler-compatibility evidence for Rust `1.90.0` through
  Rust `1.96.0`, while continuing to recommend the latest stable Rust for new
  deployments.
- Hardened wrapped-line decode prefix checks with checked offset arithmetic.
- Made `ct::CtEngine::decode_slice_staged_clear_tail` report
  `DecodeError::StagingTooSmall` when the private staging buffer, rather than
  the caller output buffer, is undersized.
- Tightened `BackendPolicy::HighAssuranceScalarOnly` so it also requires a CT
  result gate classified as a hardware speculation barrier.
- Reduced legacy whitespace decode traversal drift by sharing the byte
  iterator used by validation and decode.
- Added a guarded transfer when converting `SecretBuffer` into
  `ExposedSecretString`, plus documentation for cleanup-boundary escape hatches
  and CT loop guard debug/release behavior.

## Commit Range

- Previous tag: `v1.0.0`
- Release tag: `v1.0.1`
- Release date: `2026-05-29`

## Commits

### Security / Hardening

- `3e97852` Address 1.0.1 pentest findings

### Documentation

- `f3c657f` Add README artwork
- `5fe41a0` removed logo README
- `34f1739` Plan 1.0 source layout series

### Other Changes

- `9a79568` Lower MSRV to Rust 1.90

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.1.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
