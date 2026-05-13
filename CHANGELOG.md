# Changelog

## 0.1.0 - Unreleased

- Initial `no_std` scalar crate scaffold.
- Added strict standard and URL-safe Base64 engines.
- Added caller-owned encode/decode buffers and in-place decode.
- Added in-place encoding.
- Added optional `alloc` vector helpers.
- Added `std::io::Write` streaming encoder behind the `stream` feature.
- Added checked encoded-length helpers.
- Added exact decoded-length helpers.
- Hardened decode errors to report absolute input indexes.
- Added project plan, security policy, local gates, CI, dependency policy, SBOM script, and reproducible build script.
