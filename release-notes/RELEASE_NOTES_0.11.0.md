# base64-ng 0.11.0 Release Notes

Status: released

## Summary

- Started the next development cycle after the `0.10.0` release.
- Began the verification and panic-policy hardening milestone, with Kani
  compatibility and replacement-evidence policy as the first focus.
- Added a dedicated Kani verification policy document explaining the current
  Rust `1.95` compatibility skip and the accepted `v0.11` outcomes.
- Added focused Kani proof harnesses for scalar `decode_chunk` output bounds
  and bit-packing agreement with decoded 6-bit values.
- Added deterministic scalar `decode_chunk` bit-packing tests that exhaustively
  cover all one-byte and two-byte padded inputs plus a representative
  full-quantum grid.
- Reworked unpadded scalar tail decode and padded-length inspection to use
  slice-pattern and checked slice APIs instead of direct length-derived
  indexing.
- Added Kani proof harnesses for unpadded tail validation and decode output
  bounds.
- Reworked constant-time-oriented unpadded validation/decode reads and padding
  length inspection to use checked quad/tail helpers instead of direct
  length-derived indexing.
- Added an internal bounds invariant document and release metadata checks so
  remaining safe indexing is tied to explicit evidence before `v1.0`.
- Added a constant-time assembly review checklist and release metadata checks
  for generated-code evidence.
- Added an opt-in bounded fuzz smoke mode that records per-target release
  evidence without making normal CI runs expensive.
- Fixed strict in-place decode prevalidation so malformed padding placement
  reports the same recoverable errors as slice and vector decode.
- Added a dedicated profile/custom-alphabet fuzz target for MIME, PEM,
  bcrypt-style, `crypt(3)`-style, and caller-defined alphabets.
- Documented the accepted `v0.11` Kani verifier exception and the replacement
  release evidence required while local Kani remains behind Rust `1.95`.
- Hardened dependency-free equal-length buffer comparisons with an optimizer
  barrier in the byte-difference fold while keeping the API documented as
  constant-time-oriented best effort.
- Clarified public docs for constant-time-oriented buffer comparisons: length
  mismatch returns immediately and compared lengths must be public or
  caller-normalized.
- Made `runtime::BackendReport::unsafe_boundary_enforced` a conservative
  compile-time posture signal that is false for `simd` builds instead of an
  unconditional constant.
- Hardened `LineWrap::new` so zero-length wrapping is rejected at construction
  time; checked construction remains available for untrusted configuration.
- Clarified that inactive SIMD prototypes currently zero output with SIMD and
  then use scalar encoding, so prototype equivalence evidence validates
  scaffolding only and not vectorized Base64 correctness.
- Added `candidate_detection_mode` to runtime backend reports and snapshots so
  audit logs distinguish runtime CPU probing from compile-time target-feature
  reporting on `no_std` and other compile-time-only targets.
- Normalized the SSSE3/SSE4.1 prototype test gate to use the same prioritized
  detected-candidate policy as the AVX-512, AVX2, and NEON prototype tests.
- Documented the intentionally narrow SIMD intrinsic imports and added
  `docs/SIMD_ACTIVATION_CHECKLIST.md` for future accelerated dispatch work.
- Wiped `EncoderReader` and `DecoderReader` stack input buffers before
  propagating underlying `Read` errors.

## Commit Range

- Previous tag: `v0.10.0`
- Release tag: `v0.11.0`
- Release date: `2026-05-17`

## Commits

### Security / Hardening

- `def1859` Harden scalar decode chunk verification
- `18ed6c3` Harden scalar tail decode bounds
- `6e15e9c` Harden constant-time decode reads
- `deaa99a` Harden buffer equality fold

### Documentation

- `af5ea75` Document internal bounds invariants
- `9160fb8` Document constant-time assembly review
- `d00d10e` Document Kani verifier exception
- `b331c81` Release 0.11.0

### Verification

- `af56ea6` Add fuzz smoke evidence gate
- `6a92446` Add profile fuzz coverage

### Other Changes

- `f2eb15e` Start 0.11 verification cycle

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.11.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
