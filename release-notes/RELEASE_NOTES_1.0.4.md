# base64-ng 1.0.4 Release Notes

Status: released

## Summary

- Continued the `1.0.x` source-layout series by splitting stack-backed
  `EncodedBuffer`/`DecodedBuffer`, exposed array wrappers, `SecretBuffer`, and
  exposed secret wrappers into `src/buffers.rs` while preserving public root
  exports and API behavior.
- Excluded local README image assets from the published crate package and
  switched the README image to a GitHub raw URL so repository visuals do not
  inflate crates.io artifacts.
- Tightened runtime CT gate reporting on AArch64 by adding
  `CtGatePosture::HardwareSpeculationBarrierUnattested`; the built-in
  `HighAssuranceScalarOnly` policy now requires an attested hardware
  speculation barrier and no longer treats emitted AArch64 CSDB hint code as
  sufficient without platform evidence.
- Kept stack-backed buffer length invariants module-owned after the
  `src/buffers.rs` split by routing crate-internal writes through checked
  `set_filled` methods instead of exposing visible lengths crate-wide.
- Added explicit security notes to idiomatic `TryFrom` and `FromStr`
  conversions for `DecodedBuffer` and `SecretBuffer`, clarifying that those
  conversions use the strict standard decoder rather than the
  constant-time-oriented `ct` decoder.

## Commit Range

- Previous tag: `v1.0.3`
- Release tag: `v1.0.4`
- Release date: `2026-05-30`

## Commits

### Documentation

- `1bed122` Exclude README images from crate package
- `d3d135d` Document CT caveats for buffer conversions

### Verification

- `d813770` Clarify unattested AArch64 CT gate posture

### Other Changes

- `b0c0a01` Prepare 1.0.4 buffer module split
- `d381abd` Keep buffer length invariants encapsulated

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.4.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
