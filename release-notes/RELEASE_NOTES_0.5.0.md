# base64-ng 0.5.0 Release Notes

Status: released

## Summary

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

## Commit Range

- Previous tag: `v0.4.1`
- Release tag: `v0.5.0`
- Release date: `2026-05-14`

## Commits

### Added

- `67537e9` Add AVX-512 VBMI candidate reporting
- `b20bd57` Add candidate CPU features to backend reports
- `b55ee57` Add structured backend report snapshots
- `032fc3f` Add inactive AVX-512 encode prototype
- `24af282` Add SIMD feature bundle compile checks
- `a0ec7da` Add NEON SIMD feature bundle check
- `a8d351b` Keep tokio feature inert until admitted
- `8bb12f8` Expand zero-dependency roadmap

### Security / Hardening

- `ede2ef9` Harden macOS CI cargo resolution
- `f59d883` Harden SIMD candidate feature reporting
- `f3dff73` Harden unsafe boundary validation
- `adec375` Harden 0.5.0 pentest findings

### Documentation

- `a9e2d95` Document streaming buffer cleanup
- `a006099` Document 0.5 release evidence checks
- `7f79cdd` Require SIMD safety docs in release package
- `5577bd7` Clarify reserved async feature docs
- `73a14c5` Package release gate scripts
- `df2652b` Package release policy inputs
- `0c253ec` Validate packaged release script metadata
- `4646593` Document async feature admission policy
- `e7575a8` Clarify README admission-gated roadmap
- `fa6da38` Document dependency admission policy
- `b86d990` Prepare 0.5.0 release

### Verification

- `b504666` Support toolchain shorthand in CI cargo wrapper
- `8323c08` Fuzz DecoderReader adjacent payload boundaries
- `ac62ac1` Fuzz DecoderReader fragmented sources
- `19b766b` Add local backend evidence script
- `bc89c3f` Test stream decoder inner access after cleanup
- `a854f47` Run NEON SIMD bundle check in CI

### Other Changes

- `2b88698` Start 0.5.0 development cycle
- `27962f7` Format runtime backend assertions
- `4c48b05` Clear streaming decoder buffers on drop
- `30eb1f4` Clear streaming encoder reader queue on read
- `08bfbbe` Confine architecture feature gates to simd boundary
- `a302189` Check reserved feature placeholders
- `54cd8df` Validate reserved feature dependency graphs
- `9f1d18d` Require reserved features to stay inert
- `e7b65bb` Align 0.5 roadmap with feature admission gate

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.5.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
