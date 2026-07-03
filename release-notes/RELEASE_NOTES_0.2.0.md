# base64-ng 0.2.0 Release Notes

Status: released

## Summary

- Started the next development cycle after the `0.1.0` release.
- Documented the runtime scalar API expectation that malformed input and size
  errors return `Result` or `Option` instead of panicking.
- Added focused panic-safety regression tests for runtime scalar encode and
  decode error paths.
- Expanded bounded-memory documentation for checked length helpers and
  caller-owned decode buffers.
- Added a migration guide for projects moving from the `base64` crate.
- Added explicit legacy decode APIs that ignore ASCII transport whitespace while
  keeping alphabet and padding checks strict.
- Added exhaustive invalid-byte position tests for strict standard and URL-safe
  decoding.
- Added the constant-time decode API design document and verification bar.
- Added isolated `cargo-fuzz` harnesses for arbitrary decode input, in-place
  decode, and stream chunk-boundary behavior.
- Added a fuzz-only differential harness against the established `base64` crate
  for canonical encode/decode behavior.
- Added release evidence documentation for audit, license, SBOM, fuzz-only
  dependency, and reproducibility review.
- Added `scripts/check_fuzz.sh` and wired fuzz-only dependency checks into the
  stable release gate.
- Hardened release metadata validation so fuzz-only files cannot enter the
  published crate package.
- Added legacy whitespace decode regression tests for original-index errors and
  in-place decode parity.
- Aligned README and plan wording with the implemented stream and isolated fuzz
  harness status.
- Added CI coverage for fuzz-only dependency audit and license policy checks.
- Added cross-platform CI coverage for all-feature test runs.

## Commit Range

- Previous tag: `v0.1.0`
- Release tag: `v0.2.0`
- Release date: `2026-05-13`

## Commits

### Added

- `0358f18` Add base64 migration guide
- `3047c3f` Align roadmap with implemented gates

### Security / Hardening

- `1777b83` Harden legacy whitespace decode tests

### Documentation

- `da4ce5b` Document runtime panic safety
- `8057bff` Document bounded memory usage
- `0a66528` Document constant-time decode design
- `872938e` Document release evidence review
- `bbec398` Prepare 0.2.0 release

### Verification

- `ff10a0c` Add explicit legacy whitespace decode
- `f8609ba` Expand malformed byte tests
- `33e6a41` Add fuzz harness scaffold
- `95865c3` Add differential fuzz harness
- `ff02314` Gate fuzz dependency evidence
- `d7f8a2b` Guard package metadata against fuzz files
- `dcbfcb2` Audit fuzz dependencies in CI
- `854f592` Test all features across platforms

### Other Changes

- `247a9e7` Start 0.2.0 development cycle

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.2.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
