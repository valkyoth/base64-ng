# base64-ng 1.0.7 Release Notes

Status: released

## Summary

- Enabled the current full no-default-features Kani harness set on the pinned
  Rust `1.90.0` toolchain with `cargo-kani 0.67.0`.
- Raised Kani harness unwind bounds for the fixed 64-step
  constant-time-oriented alphabet scanner and slice loops.
- Gated inline assembly cleanup and constant-time result barriers out of Kani
  runs so the verifier models the compiler-fence fallback path instead of
  rejecting unreachable assembly.
- Updated Kani documentation and trust-dashboard wording to distinguish the
  now-clean bounded harness set from a whole-crate or cryptographic
  formal-verification claim.
- Strengthened constant-time-oriented byte accumulation through a non-inlined
  volatile helper, added AArch64 CSDB attestation posture reporting through an
  explicit custom cfg, exposed a programmatic memory-locking posture method,
  and documented streaming decoder partial-output semantics more prominently.
- Updated unsafe-boundary validation and unsafe-site documentation for the
  reviewed constant-time accumulator helper.

## Commit Range

- Previous tag: `v1.0.6`
- Release tag: `v1.0.7`
- Release date: `2026-06-07`

## Commits

### Security / Hardening

- `d942683` Harden CT posture reporting and docs
- `0ef3e91` Update unsafe boundary for CT accumulator

### Documentation

- `803b951` Prepare 1.0.7 release candidate

### Verification

- `98fbee2` Enable Kani proof gate on Rust 1.90
- `880340f` Keep AArch64 attestation out of all-features
- `a2b0b20` Route macOS CI through verification script

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.7.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
