# base64-ng 2.0.2

`base64-ng` 2.0.2 is a synchronized maintenance release. The runtime
implementation and public 2.0 API are unchanged.

## Changes

- Updates the active release toolchain to Rust 1.98.0 and retains verified
  compatibility back to the Rust 1.90.0 MSRV.
- Updates `serde_json` test/integration coverage to 1.0.151 and the active fuzz
  differential oracle to `base64 0.23.1`. The exact `base64 0.23.0`
  performance/protocol evidence oracle remains pinned so historical records
  stay reproducible.
- Updates cargo-nextest and the immutable GitHub Action pins used for cache and
  release-tool installation.
- Prevents the deliberately aborting assurance-test child from generating
  local Unix core dumps and desktop crash reports.
- Uses runtime-generated password-record fixtures and opaque PEM fuzz mismatch
  diagnostics to avoid credential-shaped constants and input disclosure in
  development tooling.
- Synchronizes all Rust companion crates and
  `@valkyoth/base64-ng-wasm-loader` at 2.0.2.

No codec behavior, public API contract, SIMD admission decision,
secret-processing boundary, or MSRV change is included.
