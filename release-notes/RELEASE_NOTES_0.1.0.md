# base64-ng 0.1.0 Release Notes

Status: released

## Summary

- Initial `no_std` scalar crate scaffold.
- Added strict standard and URL-safe Base64 engines.
- Added caller-owned encode/decode buffers and in-place decode.
- Added in-place encoding.
- Added stable compile-time encoding into caller-sized arrays.
- Added optional `alloc` vector and encoded string helpers.
- Added `std::io::Write` and `std::io::Read` streaming encoders behind the `stream` feature.
- Added `std::io::Write` streaming decoder behind the `stream` feature.
- Added `std::io::Read` streaming decoder behind the `stream` feature.
- Added checked encoded-length helpers.
- Added exact decoded-length helpers.
- Changed public encoded-length helpers to return recoverable overflow errors
  instead of panicking.
- Hardened decode errors to report absolute input indexes.
- Hardened scalar encode to avoid input-derived alphabet table indexes.
- Hardened alphabet decode to avoid branch-heavy match ladders.
- Hardened `decode_vec` to validate input before allocating decoded output.
- Optimized padding validation to avoid redundant scans on malformed inputs.
- Hardened stream decoders to preserve reader boundaries after terminal padding.
- Added Miri support in CI and the local release gate when installed.
- Added project plan, security policy, local gates, CI, dependency policy, SBOM script, and reproducible build script.

## Commit Range

- Previous tag: none
- Release tag: `v0.1.0`
- Release date: `2026-05-13`

## Commits

### Added

- `a54e70c` Add checked encoded length handling
- `3585912` Add exact decoded length helpers
- `a4a60ba` Add in-place encoding
- `5caa13b` Add std stream encoder
- `75c0bfc` Add std stream encoder reader
- `aad4f52` Add std stream decoder writer
- `46fb6e4` Add std stream decoder reader
- `2017112` Add const array encoding
- `56a5a70` Add alloc encode string helper
- `82de879` Refresh implemented roadmap status
- `dc24c5e` Add constant-time decode roadmap
- `9cd6c27` Avoid redundant padding scans

### Security / Hardening

- `75cf53f` Harden decode errors and add alloc helpers
- `af50faf` Harden padding validation tests
- `dd7f4ca` Harden package metadata validation
- `ae60a9f` Harden strict decode tests
- `4a2a668` Forbid unsafe code in scalar crate
- `2a1f5d8` Expand security roadmap
- `cbbd11c` Harden scalar encode alphabet mapping

### Documentation

- `f1de771` Document minimal dependency policy
- `ecd6452` Add contributor and release docs
- `e1f4535` Document const encoding contract
- `0ff1be6` Document local Miri verification
- `9da5740` Document publish dry-run preflight
- `4a6d826` Prepare 0.1.0 release

### Verification

- `3b0fc29` Add public API doctests
- `ce017c3` Test fragmented stream readers
- `4d3d2eb` Add optional local Miri gate
- `693a526` Add long round-trip equivalence tests
- `f101a92` Skip expensive sweeps under Miri
- `c3a9d22` Test mixed alphabet rejection

### Other Changes

- `0b6aa28` first commit
- `d5bb688` Initial secure base64-ng scaffold
- `f22031f` Enforce zero external dependency policy
- `6c6211d` Normalize license files for GitHub detection
- `b6bc4fd` Refactor stream trailing input errors
- `1f1e551` Gate no_std library builds
- `5bb45cc` Fix repository metadata
- `cc54728` Validate repository metadata
- `a3d65a6` Preserve stream decoder reader boundaries
- `563b1b8` Validate decode vec before allocation
- `19a3476` Reduce alphabet decode branching
- `afbb995` Make encoded length overflow recoverable

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.1.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
