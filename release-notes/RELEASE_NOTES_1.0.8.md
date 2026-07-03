# base64-ng 1.0.8 Release Notes

Status: released

## Summary

- Made stream decoder queue-overflow paths latch their failed state, matching
  the encoder fail-closed behavior for unreachable internal queue capacity
  errors.
- Added `DecodeErrorKind` and `DecodeError::kind()` so applications can log
  strict decode error classes without logging input-derived bytes or indexes.
- Split AArch64 CSDB attestation reporting into a distinct
  `hardware-speculation-barrier-build-asserted` posture so audit logs preserve
  the operator-attestation boundary.
- Hardened CI toolchain bootstrap by requiring runner-provided `rustup` and
  `cargo` instead of downloading and executing `sh.rustup.rs` during CI.
- Updated fuzz, dudect, and performance harness path dependency metadata to
  `1.0.8`.

## Commit Range

- Previous tag: `v1.0.7`
- Release tag: `v1.0.8`
- Release date: `2026-06-09`

## Commits

### Security / Hardening

- `3be876d` Harden audit logging and stream fail-closed paths

### Documentation

- `6d2f4fa` Prepare 1.0.8 release candidate

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.8.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
