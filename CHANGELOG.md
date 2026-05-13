# Changelog

## 0.2.0-alpha.0 - Unreleased

- Started the next development cycle after the `0.1.0` release.
- Documented the runtime scalar API expectation that malformed input and size
  errors return `Result` or `Option` instead of panicking.
- Added focused panic-safety regression tests for runtime scalar encode and
  decode error paths.
- Expanded bounded-memory documentation for checked length helpers and
  caller-owned decode buffers.

## 0.1.0 - 2026-05-13

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
