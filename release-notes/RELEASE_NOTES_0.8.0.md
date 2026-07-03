# base64-ng 0.8.0 Release Notes

Status: released

## Summary

- Started the next development cycle after the `0.7.0` release.
- Added a dependency-free `define_alphabet!` macro that generates custom
  alphabet marker types and validates their 64-byte tables at compile time.
- Added a SIMD admission manifest and release-gate validation that keeps active
  hardware acceleration blocked until backend-specific evidence is recorded.
- Extended backend evidence capture to write a manifest with toolchain
  metadata, command status, and captured-output checksums.
- Added backend evidence capture to the stable release gate and made packaged
  release metadata explicitly require the SIMD admission manifest.
- Added opt-in performance evidence capture with benchmark output and manifest
  generation under `target/release-evidence/perf/`.
- Added `TryFrom<&str>` and `TryFrom<&[u8]>` for `SecretBuffer` using strict
  standard padded Base64.
- Documented the existing `Clone`, `Copy`, `Debug`, `Eq`, and `PartialEq`
  behavior for named `Profile` values with regression coverage.
- Extended `SecretBuffer` cleanup to best-effort wipe vector spare capacity
  with an audited volatile helper when `alloc` is enabled.
- Added a release-gated documentation version validator so README, changelog,
  and SIMD release-status docs cannot drift from `Cargo.toml`.
- Tightened CI SIMD feature-bundle setup so the wasm `simd128` reserved build
  runs instead of being skipped in the main checks job.
- Extended the Miri check to write release evidence artifacts and a manifest
  when nightly Miri is installed.
- Refreshed the trust dashboard to reference the current SBOM script, Miri
  evidence artifacts, and `SecretBuffer` spare-capacity cleanup posture.
- Extended SBOM generation to write a release evidence manifest with tool
  versions, commands, and artifact checksums.
- Hardened `scripts/stable_release_gate.sh release` so stable release mode
  refuses pre-release Cargo versions.
- Made Kani's expected old-compiler skip path quiet while preserving full logs
  for real verifier failures.
- Added const policy introspection helpers for engines, named profiles, and
  constant-time-oriented decoders.
- Added const `LineWrap::checked_new` and `LineWrap::is_valid` helpers for
  defensive construction of line-wrapping policies.
- Added dependency-free `Default` and `From<Engine<_, _>>` interop for
  unwrapped `Profile` values.
- Added checked wrapped-length helpers for MIME/PEM-style output length
  calculations.
- Added `Profile::checked_new` and `Profile::is_valid` for defensive
  construction of wrapped profiles.
- Tightened `SecretBuffer::from_vec` to wipe vector spare capacity immediately
  when wrapping caller-provided vectors.
- Routed `SecretBuffer::from_slice` through the same spare-capacity cleanup
  path as `SecretBuffer::from_vec`.
- Added `SecretBuffer::constant_time_eq` for dependency-free,
  constant-time-oriented comparison of equal-length secret buffers.
- Changed `SecretBuffer` equality to use the same constant-time-oriented
  equal-length comparison helper.
- Added `EncodedBuffer::constant_time_eq` and routed `EncodedBuffer` equality
  through the same constant-time-oriented equal-length comparison helper.
- Added `SecretBuffer::into_exposed_vec` as an explicit owned interop escape
  hatch. It now returns `ExposedSecretVec`; raw `Vec<u8>` extraction requires
  the explicit unprotected escape hatch.
- Added `DecodedBuffer` plus `Engine::decode_buffer` and
  `Profile::decode_buffer` for no-alloc stack-backed decoded output.
- Added `ct::CtEngine::decode_buffer` for constant-time-oriented no-alloc
  stack-backed decoded output.
- Added `TryFrom<&str>` and `TryFrom<&[u8]>` for `DecodedBuffer<CAP>` using
  strict standard padded Base64.
- Added `into_exposed_array` escape hatches for `EncodedBuffer` and
  `DecodedBuffer`.
- Added `DecodedBuffer::as_utf8` for fallible no-alloc decoded-text views.
- Added `TryFrom<&str>` and `TryFrom<&[u8]>` for `EncodedBuffer<CAP>` using
  strict standard padded Base64 encoding.
- Added `Engine::decode_buffer_legacy` for explicit legacy-whitespace no-alloc
  decoded output.
- Added `Engine::encode_wrapped_buffer` and `Engine::decode_wrapped_buffer`
  for strict line-wrapped no-alloc stack-backed output.
- Added explicit `SecretBuffer` helpers for wrapped encode/decode and legacy
  whitespace decode.
- Added strict line-wrapped in-place decode helpers, including clear-tail
  variants and profile-level forwarding for wrapped MIME/PEM-style profiles.
- Extended in-place fuzz coverage to compare strict line-wrapped in-place
  decoding against allocated wrapped decoding.
- Extended decode fuzz coverage to compare strict line-wrapped slice
  encode/decode helpers against their allocated wrapped helpers.
- Extended wrapped fuzz and regression coverage for unpadded wrapped profiles
  and accepted trailing line endings.
- Clarified that the `0.8` release remains scalar-only unless a full
  SIMD admission evidence package lands in the same release series.
- Added doctested examples for wrapped and profile-level in-place decode APIs.
- Refreshed SIMD release-evidence wording so `0.8` docs consistently describe
  the current scalar-only admission posture.
- Corrected release-evidence unsafe-boundary wording to include the audited
  scalar-side volatile wipe helpers.
- Added `From<Vec<u8>>` for `SecretBuffer` so owned sensitive bytes can move
  into the redacted wrapper without copying.
- Added `From<String>` for `SecretBuffer` so owned sensitive text can move into
  the redacted wrapper without copying initialized bytes.
- Added `SecretBuffer::try_into_exposed_string` as an explicit owned UTF-8 text
  interop escape hatch that preserves redaction on invalid UTF-8.
- Added `SecretBuffer::expose_secret_utf8` as an explicit fallible borrowed
  UTF-8 view for secret text.
- Added direct byte-slice and byte-string literal equality for `EncodedBuffer`,
  `DecodedBuffer`, and `SecretBuffer` using their constant-time-oriented
  comparison helpers.
- Added reverse operand-order byte-slice and byte-string literal equality for
  `EncodedBuffer`, `DecodedBuffer`, and `SecretBuffer`.
- Added string equality for `EncodedBuffer`, `DecodedBuffer`, and
  `SecretBuffer` in either operand order using the same
  constant-time-oriented comparison helpers.
- Added owned `String` equality for `EncodedBuffer`, `DecodedBuffer`, and
  `SecretBuffer` in either operand order under the `alloc` feature.
- Added `From<EncodedBuffer<CAP>>` and `From<DecodedBuffer<CAP>>` for
  `SecretBuffer` under the `alloc` feature.
- Added `remaining_capacity()` to `EncodedBuffer` and `DecodedBuffer` for
  no-alloc stack sizing checks.
- Added `is_full()` to `EncodedBuffer` and `DecodedBuffer`.
- Fixed the constant-time-oriented generic decoder to scan the selected
  alphabet for every symbol instead of assuming standard ASCII ranges for
  custom alphabets.

## Commit Range

- Previous tag: `v0.7.0`
- Release tag: `v0.8.0`
- Release date: `2026-05-16`

## Commits

### Added

- `a010c6a` Add custom alphabet macro
- `b2de67b` Add SIMD admission manifest
- `8a7ea5a` Add SecretBuffer TryFrom decoders
- `a9320bb` Add Base64 policy introspection helpers
- `41c073e` Add checked LineWrap helpers
- `c2b1c7f` Add unwrapped Profile interop
- `1c58c7c` Add checked wrapped length helpers
- `3e90027` Add checked Profile construction
- `a02d14e` Add SecretBuffer constant-time comparison
- `e28fd88` Add stack-backed decoded buffer
- `ab89512` Add constant-time decoded buffer helper
- `2bc16fd` Add DecodedBuffer strict standard conversions
- `e2837dc` Add stack buffer array escape hatches
- `5389832` Add DecodedBuffer UTF-8 view
- `009a939` Add EncodedBuffer strict standard conversions
- `5035b0e` Add legacy decoded buffer helper
- `7ceb11e` Add wrapped stack buffer helpers
- `96895db` Add wrapped in-place decode helpers
- `42e3389` Add SecretBuffer owned vector conversion
- `3438347` Add SecretBuffer owned string conversion
- `e9cda3d` Add stack buffer fullness helpers
- `ed50eb1` Add SecretBuffer string exposure helper
- `2b4af52` Add SecretBuffer UTF-8 reveal helper
- `f299546` Add byte-slice comparisons for redacted buffers
- `fb1ba41` Add symmetric byte comparisons for redacted buffers
- `e0d0898` Add string comparisons for redacted buffers
- `de06450` Add owned string comparisons for redacted buffers
- `87ee1e7` Add stack buffer conversion to SecretBuffer

### Security / Hardening

- `19610dd` Wipe SecretBuffer spare capacity
- `aed94fa` Wipe SecretBuffer spare capacity on wrap
- `ad8a263` Correct release unsafe-boundary wording

### Documentation

- `9a026ab` Run backend evidence in release gate
- `024b5c6` Gate release documentation versions
- `2bfc165` Capture Miri release evidence
- `1b97a9b` Capture SBOM release evidence
- `1c2991a` Reject prerelease versions in release gate
- `c508412` Document wrapped in-place decode examples
- `e5cab78` Document redacted buffer interop posture
- `dca8eea` Prepare 0.8.0 release candidate

### Verification

- `34a4785` Capture backend evidence manifests
- `108e1da` Capture performance evidence manifests
- `71ff111` Exercise wasm SIMD bundle in CI
- `aae0f4a` Refresh trust dashboard evidence
- `d601de5` Quiet expected Kani compiler skips
- `1bb8a08` Add explicit SecretBuffer vector escape hatch
- `51785db` Add explicit profile secret helpers
- `3f89de6` Add wrapped in-place fuzz coverage
- `c4d9ec2` Add wrapped slice fuzz coverage
- `33911d0` Refresh SIMD evidence wording for 0.8
- `9b1584c` Add stack buffer remaining capacity helpers

### Other Changes

- `ad912e8` Start 0.8 development cycle
- `58c3722` Cover profile trait behavior
- `52fb27f` Reuse SecretBuffer spare cleanup for slices
- `4177416` Use constant-time comparison for SecretBuffer equality
- `73f62cd` Use constant-time comparison for EncodedBuffer equality
- `ccf749c` Extend wrapped edge-case coverage
- `a3c75ba` Clarify scalar-only 0.8 SIMD scope
- `361e0e9` Clarify v0.8 interop roadmap status
- `81eb666` Fix generic constant-time alphabet decoding

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.8.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
