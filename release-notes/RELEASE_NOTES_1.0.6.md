# base64-ng 1.0.6 Release Notes

Status: released

## Summary

- Added alloc-gated top-level `base64_ng::encode` and `base64_ng::decode`
  convenience wrappers for strict standard padded Base64 migration use cases.
- Added alloc-gated `ct::CtEngine::decode_vec` and `decode_secret` helpers so
  sensitive payload callers have an owned constant-time-oriented decode path
  that clears failed allocations and can return a redacted `SecretBuffer`.
- Added public `base64_ng::constant_time_eq` for explicit public-length
  best-effort equal-length scans, while keeping docs clear that it is not a
  formally verified MAC/password/token comparison primitive.
- Expanded README and crate-level cookbook examples for CT owned secret decode
  and comparison ergonomics.
- Strengthened idiomatic `TryFrom`/`FromStr` documentation for decoded and
  secret buffers so callers know those conversions always use strict standard
  Base64 and should use explicit engines or profiles for other alphabets.
- Addressed 1.0.6 audit follow-up by making stream decoder over-reporting
  fail closed like the stream encoder, restoring `wipe_tail` invariant checks,
  documenting CT owned-decode transient plaintext behavior, and adding
  `ct::CtEngine::decode_secret_staged` for stack-staged owned secret decode.
- Kept `serde` deferred as a future optional integration candidate instead of
  adding an external dependency to the `1.0.x` line.

## Commit Range

- Previous tag: `v1.0.5`
- Release tag: `v1.0.6`
- Release date: `2026-05-31`

## Commits

### Security / Hardening

- `6cac1b5` Address 1.0.6 pentest follow-ups

### Other Changes

- `45ef8b6` Prepare 1.0.6 secure ergonomics

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.6.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
