# base64-ng 1.0.5 Release Notes

Status: released

## Summary

- Combined the remaining `1.0.x` source-layout cleanup into one final
  maintenance release before the community test pause.
- Split constant-time-oriented decode, validation, masks, comparison helpers,
  and CT result-gate barriers into `src/ct.rs` while preserving the public
  `base64_ng::ct::*` API surface.
- Split length and wrapping policy helpers into `src/length.rs`, strict and
  legacy wrapping internals into `src/wrap.rs`, scalar encode/decode internals
  into `src/scalar.rs`, and public error types into `src/errors.rs`.
- Updated release-gate scripts and unsafe-boundary checks for the new module
  layout without adding runtime dependencies or changing behavior.
- Addressed follow-up audit feedback by making secret-string conversion
  panic-safe without `mem::forget`, guarding tail cleanup bounds, adding
  failed-state diagnostics to the stream encoder, and documenting CT platform
  posture and strict non-CT secret conversion paths more explicitly.
- Refreshed documentation and release metadata to mark `1.0.5` as the final
  planned `1.0.x` cleanup before parking feature work for broader testing.

## Commit Range

- Previous tag: `v1.0.4`
- Release tag: `v1.0.5`
- Release date: `2026-05-30`

## Commits

### Added

- `17e014e` Address 1.0.5 audit follow-ups

### Other Changes

- `fff34cb` Prepare 1.0.5 source layout cleanup

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.5.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
