# base64-ng 1.2.2 Release Notes

Status: released

## Summary

- Added explicit infallible encode convenience helpers for ordinary
  byte-to-Base64 paths: `Engine::encode_vec_infallible`,
  `Engine::encode_string_infallible`, matching `Profile` helpers, and the
  top-level strict standard `base64_ng::encode_infallible`.
- Documented the panic contract for infallible encode helpers and kept the
  existing fallible APIs as the recommended path for untrusted length metadata,
  constrained allocation environments, and recoverable-error code paths.

## Commit Range

- Previous tag: `v1.2.1`
- Release tag: `v1.2.2`
- Release date: `2026-06-23`

## Commits

### Added

- `ecfbac3` Add infallible encode helpers

### Security / Hardening

- `0d85eeb` Harden 1.2.2 pentest follow-ups
- `579019b` Wipe staged locked output on allocation failure

### Documentation

- `ca42dd1` Align 1.2.2 release documentation

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.2.2.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
