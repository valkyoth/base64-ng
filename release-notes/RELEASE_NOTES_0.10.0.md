# base64-ng 0.10.0 Release Notes

Status: released

## Summary

- Added a dedicated public API audit checklist for the `v0.10`
  release-candidate audit-preparation milestone.
- Added release-gated public API audit validation so stable releases cannot
  leave audit rows marked as `review pending`.
- Classified profile and validation-only APIs as `v1.0` stable candidates
  with explicit audit rationale and stability boundaries.
- Classified stack-backed buffers and `SecretBuffer` as documented `v1.0`
  security boundaries with explicit ownership and cleanup limits.
- Classified in-place APIs and custom alphabet helpers for the `v1.0` audit,
  including explicit encode-to-back/decode-to-front and fixed-scan custom
  alphabet boundaries.
- Classified stream adapters and error types for the `v1.0` audit, preserving
  fail-closed decode, checked recovery, framed-reader, localized diagnostic,
  and opaque constant-time-oriented error boundaries.
- Tightened security documentation for public ct success/failure and length
  boundaries, volatile best-effort cleanup limits, and const-array panic
  policy.
- Added optional downstream guidance for applications that combine
  caller-owned `base64-ng` buffers with their own admitted `zeroize` policy.

## Commit Range

- Previous tag: `v0.9.0`
- Release tag: `v0.10.0`
- Release date: `2026-05-17`

## Commits

### Security / Hardening

- `59e97b5` Document pentest boundary findings

### Documentation

- `a8b5d0e` Prepare 0.10.0 release
- `3ef629b` Document downstream zeroize layering

### Other Changes

- `61f4313` Start 0.10 audit cycle
- `a10d4d2` Gate public API audit status
- `80e9568` Audit profile validation APIs
- `3e2ef30` Audit sensitive buffer APIs
- `6a3f5c3` Audit in-place and alphabet APIs
- `8a3c240` Audit stream and error APIs

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.10.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
