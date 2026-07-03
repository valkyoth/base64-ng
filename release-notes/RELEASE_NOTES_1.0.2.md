# base64-ng 1.0.2 Release Notes

Status: released

## Summary

- Started the `1.0.x` source-layout series by splitting the `std::io`
  streaming adapters into `src/stream.rs` and stream integration tests into
  `tests/stream.rs` while preserving the public `base64_ng::stream::*` API
  surface.
- Added `#[must_use]` to `ct::CtEngine::decode_slice_staged_clear_tail` and
  strengthened CT documentation for staged decode, AArch64 CSDB attestation,
  RISC-V ordering-fence posture, and high-assurance comparison boundaries.
- Added a debug UTF-8 invariant check before the internal secret-string
  unchecked conversion, clarified `SecretBuffer::clear` ordering, and made
  `stream::Encoder` reject empty writes after finalization consistently with
  non-empty writes.

## Commit Range

- Previous tag: `v1.0.1`
- Release tag: `v1.0.2`
- Release date: `2026-05-29`

## Commits

### Security / Hardening

- `528d523` Address 1.0.2 pentest findings

### Other Changes

- `ce44e91` Start 1.0.2 stream module split

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.2.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
