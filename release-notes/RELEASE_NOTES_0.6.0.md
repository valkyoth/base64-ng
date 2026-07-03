# base64-ng 0.6.0 Release Notes

Status: released

## Summary

- Started the next development cycle after the `0.5.0` release.
- Added no-alloc validation-only APIs for strict and legacy profiles:
  `validate_result`, `validate`, `validate_legacy_result`, and
  `validate_legacy`.
- Added constant-time-oriented validation-only APIs:
  `ct::CtEngine::validate_result` and `ct::CtEngine::validate`.
- Added dependency-free line-wrapped encoding with `LineWrap`, `LineEnding`,
  checked wrapped-length calculation, caller-owned output APIs, clear-tail
  wrapping, and `alloc` convenience helpers.
- Added strict line-wrapped validation and decoding APIs that accept only the
  configured line ending and enforce configured-width non-final lines.
- Added custom alphabet helpers: `validate_alphabet`,
  `decode_alphabet_byte`, and `AlphabetError`.
- Added named dependency-free profiles for MIME, PEM, bcrypt-style, and
  `crypt(3)`-style Base64 through `Profile`, `MIME`, `PEM`, `PEM_CRLF`,
  `BCRYPT`, and `CRYPT`.
- Added `EncodedBuffer` and `encode_buffer` helpers for stack-backed short
  encoded output without requiring `alloc`.
- Added `SecretBuffer`, `encode_secret`, and `decode_secret` helpers for
  redacted owned sensitive output with dependency-free best-effort cleanup.
- Added `docs/TRUST.md`, `docs/SECURITY_CONTROLS.md`, and a README trust
  dashboard for adoption-focused security evidence and CWE mapping.
- Added `docs/PANIC_POLICY.md` and `scripts/validate-panic-policy.sh` to keep
  runtime panic-like sites reviewed and release-gated.
- Added `scripts/check_miri.sh` and routed CI/release Miri checks through it so
  both no-default scalar and all-features alloc/stream surfaces run under Miri
  when nightly Miri is installed.
- Added `docs/FUZZING.md` and `scripts/check_fuzz_corpus.sh` to document and
  enforce reviewed fuzz corpus handling.
- Expanded `docs/CONSTANT_TIME.md` with generated-code review requirements and
  added `scripts/validate-constant-time-policy.sh` to release-gate the current
  constant-time non-claim wording.
- Expanded gated Kani proof harness definitions for slice encode/decode,
  clear-tail decode, and in-place encode bounds while keeping execution gated
  on Kani's bundled compiler support.
- Hardened scalar chunk validation and decode helpers to use checked quad
  reads and typed `[u8; 4]` inputs instead of debug-asserted slice lengths.
- Replaced the cleanup helper's ordinary zero fill with an audited volatile
  write loop so best-effort wiping is not optimized away.
- Reduced constant-time-oriented padded terminal handling by replacing explicit
  padding-count branches with masked final-quantum validation and
  length-derived final writes.

## Commit Range

- Previous tag: `v0.5.0`
- Release tag: `v0.6.0`
- Release date: `2026-05-14`

## Commits

### Added

- `4fcc78d` Add line-wrapped encoding APIs
- `74eda10` Add strict wrapped decode profile
- `28141ec` Add custom alphabet validation helpers
- `2509fd2` Add named Base64 profiles
- `aea1cd2` Add stack encoded buffer helper
- `c4ef997` Add secret buffer cleanup helpers
- `b6709cc` Add panic policy validation
- `cf1154c` Add constant-time validation APIs
- `c060789` Mark v0.6 roadmap implementation complete

### Security / Hardening

- `e0120de` Add trust dashboard and security mapping
- `181b0af` Harden scalar chunk decoding invariants
- `6dd1990` Harden volatile wiping and ct padding

### Documentation

- `cbda505` Document and check fuzz corpus policy
- `c9e4048` Release base64-ng 0.6.0

### Verification

- `5fa6379` Strengthen Miri coverage gate
- `69709c4` Expand gated Kani proof harnesses

### Other Changes

- `fef0e78` Start 0.6 validation APIs
- `4dd60ac` Gate constant-time verification policy

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.6.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
