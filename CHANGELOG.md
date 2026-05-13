# Changelog

## 0.4.0-alpha.0 - Unreleased

- Started the next development cycle after the `0.3.0` release.

## 0.3.0 - 2026-05-13

- Started the next development cycle after the `0.2.0` release.
- Added an initial `ct` scalar decode module for caller-owned buffers. The path
  avoids secret-indexed lookup tables during Base64 symbol mapping while
  remaining explicit that it is not yet a formally verified cryptographic
  constant-time API.
- Extended fuzz coverage to check `ct` decode success/error parity against the
  strict scalar decoder.
- Extended exhaustive short canonical round-trip tests to cover the `ct`
  decoder for all one- and two-byte inputs across all built-in engines.
- Changed `ct` malformed-input reporting to non-localized sentinel errors so
  error tracking does not expose the first malformed byte position.
- Changed `ct` padding-length calculation to use mask arithmetic instead of
  branch-based equality checks.
- Added a SIMD admission policy that keeps hardware acceleration out of `0.3`
  until unsafe code isolation, dispatch behavior, fuzzing, CI, and benchmark
  evidence are ready.
- Added an isolated no-framework scalar performance comparison harness against
  the established `base64` crate.
- Added performance harness dependency audit and license checks to CI and the
  release gate.
- Documented how performance evidence should be generated and qualified for
  releases.
- Documented scalar decode throughput as an explicit optimization target rather
  than a release claim.
- Added strict and legacy in-place decode variants that clear unused buffer
  bytes on success and clear the full caller buffer on error.
- Extended in-place fuzz coverage to verify clear-tail decode result parity and
  cleanup semantics.
- Documented clear-tail decode cleanup as best-effort buffer-retention
  reduction rather than a formal zeroization guarantee.
- Added an in-place encode variant that clears unused buffer bytes on success
  and clears the full caller buffer on error.
- Extended in-place fuzz coverage to verify clear-tail encode result parity and
  cleanup semantics.
- Added slice-output encode and decode variants that clear unused output bytes
  on success and clear the output buffer on error.
- Extended decode fuzz coverage to verify clear-tail slice encode/decode result
  parity and cleanup semantics.
- Added rustdoc examples for clear-tail slice APIs so the cleanup contract is
  visible in generated API documentation.
- Updated the roadmap to reflect the dependency-free clear-tail design instead
  of adding a default cleanup dependency.

## 0.2.0 - 2026-05-13

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
