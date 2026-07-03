# base64-ng 1.0.9 Release Notes

Status: released

## Summary

- Added `base64-ng-sanitization` as an optional companion crate for projects
  that already admit the `sanitization` crate and want direct decode helpers
  into clear-on-drop secret containers.
- Kept the core `base64-ng` package dependency-free by making the new
  integration a separate workspace member instead of a default feature.
- Added constant-time-oriented `CtDecodeSanitizationExt` helpers for
  stack-backed `SecretBytes<N>` and alloc-gated `SecretVec` outputs.
- Added `base64-ng-derive` as an optional dependency-free proc-macro companion
  crate for fixed-size `[u8; N]` tuple newtypes.
- Added `Base64Secret`, a narrow derive that generates CT staged strict
  standard Base64 parsing, strict encoding helpers, redacted `Debug`, fixed
  width comparison, and drop-time cleanup through the core wipe path.
- Added `base64-ng-serde` with explicit standard and URL-safe no-padding serde
  wrappers and `#[serde(with = "...")]` modules.
- Added `base64-ng-bytes` with `Bytes`, `Buf`, and `BufMut` helpers for
  network-service buffer integration.
- Added `base64-ng-tokio` with bounded async read/write helpers that validate
  or encode before writing output.
- Exposed `base64_ng::clear_bytes` so companion crates and downstream
  applications can reuse the reviewed best-effort cleanup primitive without
  emitting unsafe cleanup code.
- Refreshed the companion-crate publish plan so `base64-ng`,
  `base64-ng-sanitization`, `base64-ng-derive`, `base64-ng-serde`,
  `base64-ng-bytes`, and `base64-ng-tokio` publish in dependency order.
- Documented the companion-crate policy and future optional subcrate candidates
  while keeping SIMD work reserved for the `1.1` line.

## Commit Range

- Previous tag: `v1.0.8`
- Release tag: `v1.0.9`
- Release date: `2026-06-20`

## Commits

### Added

- `5cdd7bb` Add sanitization companion crate
- `e7c89a0` Add workspace crate publish helper
- `66d4482` Add derive companion crate for 1.0.9
- `e9a79c9` Add ecosystem companion crates for 1.0.9

### Security / Hardening

- `94cd837` Address 1.0.9 pentest follow-ups

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.9.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
