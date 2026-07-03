# base64-ng 0.4.0 Release Notes

Status: released

## Summary

- Released the `0.4.0` hardening and SIMD admission milestone.
- Added cross-target CI checks for the reserved `simd` feature in `no_std`
  mode across x86_64, aarch64, FreeBSD, wasm32, and Cortex-M targets.
- Added `scripts/check_targets.sh` so installed cross-target `no_std`
  `simd`-reserved builds can be checked locally and from CI.
- Documented reinstall/setup commands for cross targets, nextest, cargo-fuzz,
  Kani, and release-security tooling.
- Fixed optional release-gate tool detection to use Cargo subcommands such as
  `cargo nextest --version`.
- Added initial Kani proof harnesses for scalar length helpers and bounded
  in-place decode behavior.
- Added `scripts/check_kani.sh` so Kani proofs run when compatible and skip
  clearly when Kani's bundled compiler is behind the crate's pinned Rust.
- Added an internal scalar backend boundary so future SIMD dispatch has an
  explicit reference path without changing public behavior.
- Added backend differential tests that compare the dispatch entry points
  against the scalar reference for canonical inputs, malformed inputs, and
  undersized output buffers.
- Added the SIMD unsafe admission boundary: crate-level `deny(unsafe_code)`,
  `allow(unsafe_code)` confined to `src/simd.rs`, and a release-gate check for
  that boundary.
- Added SIMD dispatch scaffolding that detects AVX2/NEON candidates while
  keeping scalar as the only active backend until accelerated code has evidence.
- Added an inactive AVX2 fixed-block encode prototype with scalar-equivalence
  tests that run only when AVX2 is available.
- Added an inactive NEON fixed-block encode prototype with scalar-equivalence
  tests that compile only for NEON-capable ARM targets.
- Added `docs/UNSAFE.md` as a central unsafe-code inventory for current SIMD
  prototype sites and their invariants.
- Extended the unsafe-boundary validation script to require inventory entries
  for current unsafe prototype sites.
- Added `runtime::backend_report()` so callers can audit the active backend,
  detected candidate, SIMD feature status, and scalar-only security posture.
- Added `runtime::require_backend_policy()` for deployment assertions such as
  scalar-only execution and no-SIMD build requirements.
- Added `BackendPolicy::HighAssuranceScalarOnly` and
  `BackendReport::satisfies()` for captured-report policy checks.
- Added stable string identifiers and `Display` implementations for runtime
  backend, posture, and policy enums for audit logs and CI artifacts.
- Added stable key/value `Display` output for runtime backend reports and
  policy failures.
- Updated the security policy with the current unsafe boundary and runtime
  backend policy controls.
- Added `ct::CtEngine::decode_slice_clear_tail`,
  `ct::CtEngine::decode_in_place`, and
  `ct::CtEngine::decode_in_place_clear_tail` so constant-time-oriented decode
  callers can clear partially decoded output on rejected sensitive input. The
  non-clear-tail CT methods were later removed before the `1.0` stable
  boundary.
- Hardened streaming encoders to clear plaintext pending buffers on drop, and
  after pending plaintext is consumed, while preserving `finish()` and
  `into_inner()` behavior.
- Hardened CI Rust setup so macOS runners explicitly install the pinned
  toolchain before invoking Cargo.

## Commit Range

- Previous tag: `v0.3.0`
- Release tag: `v0.4.0`
- Release date: `2026-05-14`

## Commits

### Added

- `16c9b58` Add local cross-target check script
- `8df9d40` Add scalar backend boundary
- `52731ba` Add SIMD dispatch scaffold
- `8e2b777` Add inactive AVX2 encode prototype
- `7e747b4` Add runtime backend report
- `aa702b0` Add runtime backend policy assertions
- `8125e27` Add high assurance backend policy
- `0f26146` Add stable runtime enum labels
- `487d931` Add audit-friendly backend report display
- `69e8428` Add inactive NEON encode prototype
- `06cf78b` Add constant-time clear-tail decode APIs
- `c07f267` Add FreeBSD target build check

### Security / Hardening

- `dc56b82` Add SIMD unsafe boundary check
- `c930d66` Document current SIMD security boundary
- `d23aeae` Add enforced unsafe inventory
- `20a3cb1` Harden CI Rust toolchain setup

### Documentation

- `0977dd2` Document local release toolchain setup
- `184c1eb` Detect optional cargo subcommands in release gate
- `515a792` Prepare 0.4.0 release

### Verification

- `a62758a` Add cross-target no_std simd CI
- `5fba2a7` Add initial Kani proof harnesses
- `599ed68` Add backend differential tests

### Other Changes

- `cb752c5` Start 0.4.0 development cycle
- `2782457` Configure funding options in FUNDING.yml
- `3b8028c` Clear stream encoder pending buffers on drop

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.4.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
