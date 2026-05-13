# Changelog

## 0.1.0 - Unreleased

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
- Hardened decode errors to report absolute input indexes.
- Hardened stream decoders to preserve reader boundaries after terminal padding.
- Added Miri support in CI and the local release gate when installed.
- Added project plan, security policy, local gates, CI, dependency policy, SBOM script, and reproducible build script.
