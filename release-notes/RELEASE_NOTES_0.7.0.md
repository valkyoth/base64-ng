# base64-ng 0.7.0 Release Notes

Status: released

## Summary

- Started the next development cycle after the `0.6.0` release.
- Scoped `0.7.0` as a scalar-only security-evidence release; active SIMD
  dispatch remains intentionally out of scope until a later admission milestone.
- Added a release-gated SIMD admission validator that keeps active dispatch
  scalar-only until accelerated backend evidence is updated deliberately.
- Added wasm `simd128` candidate reporting and a reserved `no_std`
  feature-bundle compile check while keeping scalar as the only active backend.
- Added SSSE3/SSE4.1 candidate reporting and reserved feature-bundle compile
  evidence for older x86 CPUs before any active SIMD admission.
- Added an inactive SSSE3/SSE4.1 fixed-block encode prototype with
  scalar-equivalence tests while keeping runtime dispatch scalar-only.
- Added reserved SIMD feature-bundle compile checks to the normal local
  `scripts/checks.sh` gate so day-to-day checks match release expectations.
- Added an isolated dependency-free dudect-style timing harness for the scalar
  constant-time-oriented decoder, with compile/dependency checks in CI and
  opt-in local timing runs.
- Added release assembly evidence generation for no-default-features and
  all-features constant-time generated-code review.
- Added rustc metadata, review-focus notes, and artifact checksums to the
  generated constant-time assembly evidence.
- Added bounded Kani proof harnesses for constant-time-oriented decode result
  bounds, error cleanup, and validate/decode agreement.
- Added manifest generation for opt-in dudect-style timing evidence, including
  toolchain metadata, command parameters, raw output checksum, and result status.
- Added deterministic regression coverage that constant-time-oriented
  validation and decode agree on valid and malformed inputs across supported
  alphabets and padding modes.
- Hardened streaming adapters to wipe short-lived stack buffers used for
  temporary encoded, decoded, and read staging data.
- Documented the throughput tradeoff of the conservative custom alphabet
  encoding fallback.

## Commit Range

- Previous tag: `v0.6.0`
- Release tag: `v0.7.0`
- Release date: `2026-05-15`

## Commits

### Added

- `e856ded` Add wasm simd128 candidate gating
- `7d74c95` Add SSSE3 SSE4.1 candidate gating
- `640b567` Add SSSE3 SSE4.1 encode prototype

### Security / Hardening

- `235def5` Harden stream temporaries

### Documentation

- `fc7702b` Scope 0.7 as scalar evidence release
- `34d1107` Prepare 0.7.0 release

### Verification

- `f60af72` Add dudect timing evidence harness
- `8561062` Add constant-time assembly evidence generation
- `54b6fb3` Add assembly evidence manifest
- `688145a` Add Kani harnesses for ct decode
- `b89846f` Add dudect evidence manifest
- `603d208` Test ct validate decode agreement

### Other Changes

- `4fbd0db` Start 0.7 SIMD admission gating
- `56f42de` Run SIMD bundle checks in local gate

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.7.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
