# Changelog

## 0.5.0 - 2026-05-14

- Started the next development cycle after the `0.4.1` release.
- Added AVX-512 VBMI candidate detection to runtime backend reports while
  keeping scalar as the only active backend.
- Hardened AVX-512 candidate detection to require the full planned Base64
  feature bundle and exposed backend CPU feature requirements for audit logs.
- Added detected-candidate CPU feature requirements to runtime backend report
  formatting.
- Added `BackendReport::snapshot()` for structured runtime backend audit logs.
- Added an inactive AVX-512 fixed-block encode prototype with scalar-equivalence
  tests gated by the detected AVX-512 Base64 feature bundle.
- Added local release-gate and CI compile checks for the reserved x86 AVX2 and
  AVX-512 SIMD feature bundles under `no_std`.
- Extended stream fuzz coverage to verify padded `DecoderReader` leaves
  adjacent framed payload bytes unread.
- Extended stream fuzz coverage to compare fragmented `DecoderReader` sources
  against slice decoding when payload-boundary semantics match.
- Hardened streaming decoders to clear pending input on drop and clear decoded
  output queue bytes before they are discarded.
- Hardened streaming encoder readers to clear queued encoded output bytes before
  they are discarded.
- Added focused regression tests for decoder `finish()` and `into_inner()` paths
  after stream cleanup hardening.
- Hardened the unsafe-boundary validator so every SIMD-boundary unsafe function
  must be documented and every unsafe block must have a nearby `SAFETY:`
  explanation.
- Added release-gate validation that architecture intrinsics, CPU feature
  detection, and `target_feature` gates remain confined to `src/simd.rs`.
- Extended the SIMD feature-bundle check to compile the reserved NEON path
  under `no_std` when `aarch64-unknown-linux-gnu` is installed.
- Updated CI to install `aarch64-unknown-linux-gnu` before the SIMD
  feature-bundle check so NEON reserved-build evidence runs in automation.
- Hardened release metadata validation to require the published crate package
  to include the SIMD admission policy and unsafe inventory documents.
- Clarified 0.5 development documentation around benchmark claims and reserved
  `tokio` feature behavior.
- Added a reserved-feature placeholder check to prove `tokio`, `kani`, and
  `fuzzing` remain dependency-free compile-only features until admitted.
- Included core release/check scripts in the published package and required
  metadata validation to prove those gate assets are present.
- Included the Rust toolchain pin and cargo-deny policy in the published
  package so packaged release scripts have their required policy inputs.
- Hardened release metadata validation to require packaged release scripts to
  be executable and use the portable `#!/usr/bin/env sh` shebang.
- Strengthened reserved-feature placeholder checks with explicit per-feature
  dependency graph validation.
- Made the reserved `tokio` feature truly inert until async wrappers are
  admitted instead of enabling the existing `stream` feature.
- Hardened reserved-feature checks to require `tokio`, `kani`, and `fuzzing`
  to remain inert Cargo features.
- Updated the v0.5 roadmap to reflect the reserved async-feature admission bar
  instead of promising Tokio wrappers before dependency review.
- Added `docs/ASYNC.md` to define the admission requirements for any future
  async/Tokio API while the `tokio` feature remains inert.
- Clarified README planned-work wording so async wrappers are described as
  admission-gated rather than already scheduled functionality.
- Added `docs/DEPENDENCIES.md` to make external crate admission requirements
  explicit and packaged with release evidence.
- Expanded the roadmap with a zero-dependency "ultimate crate" backlog covering
  MIME/PEM/bcrypt profiles, custom alphabets, line wrapping, validation-only
  APIs, stack-backed short outputs, secret wrappers, CWE mapping, and
  admission-gated ecosystem integrations.
- Replaced streaming reader `VecDeque` output queues with fixed-size internal
  queues that clear consumed slots and wipe their full capacity on drop.
- Hardened the constant-time-oriented decoder to report malformed content
  through one opaque error instead of exposing invalid-byte versus
  invalid-padding categories.
- Replaced generic boolean mask generation in Base64 symbol mapping with
  integer byte-mask helpers and documented the remaining generated-code review
  requirement.
- Clarified that clear-tail APIs reduce buffer retention but do not hide public
  success, failure, or decoded-length results.
- Added SIMD admission documentation requiring future real vector paths to
  document their register-retention cleanup strategy.
- Updated security documentation for streaming wrapper buffer cleanup behavior.
- Updated release evidence documentation for stream fuzzing and reserved SIMD
  feature-bundle checks.
- Added a local backend evidence script for runtime backend reporting and gated
  SIMD prototype tests.

## 0.4.1 - 2026-05-14

- Fixed published documentation examples and harness manifests to reference
  `0.4.1` instead of stale `0.3` or `0.4.0-alpha.0` development versions.
- Updated release documentation to describe the current `0.4.1` crate status.

## 0.4.0 - 2026-05-14

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
  callers can clear partially decoded output on rejected sensitive input.
- Hardened streaming encoders to clear plaintext pending buffers on drop, and
  after pending plaintext is consumed, while preserving `finish()` and
  `into_inner()` behavior.
- Hardened CI Rust setup so macOS runners explicitly install the pinned
  toolchain before invoking Cargo.

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
