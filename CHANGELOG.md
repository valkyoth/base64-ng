# Changelog

## 1.0.0-alpha.0 - Unreleased

- Started the stable API and security-contract freeze candidate after the
  `0.12.0` stabilization release.
- Accepted the documented Kani verifier exception for the initial `1.0.0`
  contract: Kani harnesses remain in-tree and release-gated, but incompatible
  Kani compiler runs are policy skips backed by replacement evidence, not
  proofs.
- Hardened `stream::Encoder::write` so accepted input after a completed
  pending quantum continues through the current slice when buffer capacity
  allows, including preserving final 1-2 byte tails instead of forcing an
  early short write.
- Hardened `stream::Decoder::write` so direct writes process multiple complete
  Base64 quads per call, continue after completing pending input, and preserve
  final partial quads as pending input when those bytes are accepted.

## 0.12.0 - 2026-05-17

- Started the stabilization rehearsal cycle after the `0.11.0` release.
- Added a migration-guide smoke crate and release-gate check covering strict
  standard, URL-safe no-pad, MIME/PEM, legacy whitespace, custom alphabet,
  stack-buffer, secret-buffer, and stream migration examples.
- Hardened release metadata validation and the stable release gate so the
  migration smoke source and check script stay packaged and release-gated.
- Added an MSRV/toolchain policy validator covering Cargo metadata,
  `rust-toolchain.toml`, docs.rs metadata, CI install paths, target matrices,
  Kani policy, and release evidence tooling.
- Added the `v0.12` final dependency admission review, keeping optional
  ecosystem integrations deferred unless they earn separate admission evidence.
- Changed custom alphabet byte decoding to scan all 64 alphabet entries before
  returning, avoiding match-position early returns for bcrypt-style,
  `crypt(3)`-style, and caller-defined alphabets.
- Clarified that default strict decoders are not constant-time decoders and
  that secret-bearing payloads should use the `ct` module when timing posture
  matters more than localized error diagnostics.
- Changed internal stream output-queue saturation errors away from
  `InvalidInput` so bounded queue exhaustion is not reported as malformed
  caller input.
- Expanded software-only wipe documentation with the known limits of volatile
  best-effort cleanup and the recommended application-owned `zeroize` pattern
  for deployments that already admit that dependency.

## 0.11.0 - 2026-05-17

- Started the next development cycle after the `0.10.0` release.
- Began the verification and panic-policy hardening milestone, with Kani
  compatibility and replacement-evidence policy as the first focus.
- Added a dedicated Kani verification policy document explaining the current
  Rust `1.95` compatibility skip and the accepted `v0.11` outcomes.
- Added focused Kani proof harnesses for scalar `decode_chunk` output bounds
  and bit-packing agreement with decoded 6-bit values.
- Added deterministic scalar `decode_chunk` bit-packing tests that exhaustively
  cover all one-byte and two-byte padded inputs plus a representative
  full-quantum grid.
- Reworked unpadded scalar tail decode and padded-length inspection to use
  slice-pattern and checked slice APIs instead of direct length-derived
  indexing.
- Added Kani proof harnesses for unpadded tail validation and decode output
  bounds.
- Reworked constant-time-oriented unpadded validation/decode reads and padding
  length inspection to use checked quad/tail helpers instead of direct
  length-derived indexing.
- Added an internal bounds invariant document and release metadata checks so
  remaining safe indexing is tied to explicit evidence before `v1.0`.
- Added a constant-time assembly review checklist and release metadata checks
  for generated-code evidence.
- Added an opt-in bounded fuzz smoke mode that records per-target release
  evidence without making normal CI runs expensive.
- Fixed strict in-place decode prevalidation so malformed padding placement
  reports the same recoverable errors as slice and vector decode.
- Added a dedicated profile/custom-alphabet fuzz target for MIME, PEM,
  bcrypt-style, `crypt(3)`-style, and caller-defined alphabets.
- Documented the accepted `v0.11` Kani verifier exception and the replacement
  release evidence required while local Kani remains behind Rust `1.95`.
- Hardened dependency-free equal-length buffer comparisons with an optimizer
  barrier in the byte-difference fold while keeping the API documented as
  constant-time-oriented best effort.

## 0.10.0 - 2026-05-17

- Added a dedicated public API audit checklist for the `v0.10`
  release-candidate audit-preparation milestone.
- Added release-gated public API audit validation so stable releases cannot
  leave audit rows marked as `review pending`.
- Classified profile and validation-only APIs as `v1.0` stable candidates
  with explicit audit rationale and stability boundaries.
- Classified stack-backed buffers and `SecretBuffer` as documented `v1.0`
  security boundaries with explicit ownership and cleanup limits.
- Classified in-place APIs and custom alphabet helpers for the `v1.0` audit,
  including explicit encode-to-back/decode-to-front and fixed-scan custom
  alphabet boundaries.
- Classified stream adapters and error types for the `v1.0` audit, preserving
  fail-closed decode, checked recovery, framed-reader, localized diagnostic,
  and opaque constant-time-oriented error boundaries.
- Tightened security documentation for public ct success/failure and length
  boundaries, volatile best-effort cleanup limits, and const-array panic
  policy.
- Added optional downstream guidance for applications that combine
  caller-owned `base64-ng` buffers with their own admitted `zeroize` policy.

## 0.9.0 - 2026-05-17

- Started the next development cycle after the `0.8.0` release.
- Added stream adapter state-inspection helpers for pending input quanta and
  buffered reader output plus terminal padded decode blocks, improving
  framed-protocol ergonomics without adding dependencies.
- Added a dependency-free no-alloc portability smoke crate and local gate that
  builds stack-backed APIs with default features disabled across installed
  Linux, wasm32, and Cortex-M targets.
- Added stream reader `is_finished()` state helpers plus redacted `Debug`
  output for stream adapters so diagnostics can inspect buffering state without
  formatting wrapped readers or writers.
- Added framed-protocol stream regressions for fragmented padded decoder
  sources, proving terminal-padding state is reported before buffered decoded
  bytes are drained and adjacent payload bytes remain unread.
- Documented the v0.9 dependency-admission stance for deferred `tokio`,
  `serde`, `bytes`, cleanup, timing, and benchmark integrations.
- Hardened reserved-feature checks so deferred `serde`, `bytes`, cleanup,
  timing, and benchmark integration features cannot appear before dependency
  admission.
- Extended CI target-matrix coverage to run the no-alloc portability smoke
  harness for each installed `no_std` target.
- Expanded the no-alloc portability smoke harness to compile validate-only,
  legacy decode, in-place encode/decode, and constant-time-oriented in-place
  decode surfaces with default features disabled.
- Aligned the default no-alloc portability smoke target list with the main
  installed cross-target check list.
- Added host-side unit tests to the no-alloc portability smoke harness before
  cross-target compile checks.
- Added writer-side stream `try_finish()` helpers so callers can finalize and
  flush pending Base64 quanta without consuming the adapter.
- Hardened writer-side stream finalization so adapters reject later input after
  `try_finish()` succeeds.
- Added writer-side stream `is_finalized()` helpers for explicit finalized
  state inspection.
- Added stream adapter `engine()` and `is_padded()` helpers for dependency-free
  policy inspection in diagnostics and audit logs.
- Added reader-side stream `has_finished_input()` helpers so callers can
  distinguish EOF or terminal padding from fully drained buffered output.
- Expanded the no-alloc portability smoke harness to cover custom alphabets,
  checked profiles, recoverable length helpers, and stack-buffer state helpers.
- Added stream finalization failure regressions proving failed `try_finish()`
  calls do not mark adapters finalized and can be retried.
- Added `Display` for `EncodedBuffer` so stack-backed encoded output can be
  formatted without allocating, while `Debug` remains redacted.
- Added fallible `EncodedBuffer::as_utf8()` for callers that prefer
  recoverable text access even though crate-produced Base64 is ASCII by
  invariant.
- Added a stream finalization regression proving `try_finish()` retries after
  flush errors do not re-emit the terminal Base64 quantum.
- Added the matching decoder-side flush retry regression so final decoded bytes
  are not emitted twice after a failed `try_finish()` flush.
- Added a decoder-side final pending write-failure regression proving failed
  `try_finish()` calls preserve pending input and can be retried.
- Added stream writer regressions proving failed `write_all()` calls preserve
  pending encoder and decoder input until the wrapped writer succeeds.
- Documented stream retry semantics for wrapped writer failures and
  finalization flush retries.
- Added `LineEnding::as_str()` for allocation-free text inspection of wrapping
  policies.
- Added const `LineWrap::line_len()` and `LineWrap::line_ending()` accessors
  for audit-friendly wrapping policy inspection.
- Added `LineEnding::name()` and `Display` for printable wrapping-policy
  identifiers without confusing them with literal line-ending bytes.
- Added `Display` for `LineWrap` so audit logs can print wrapping policies as
  stable values such as `76:CRLF`.
- Added const `Profile::line_len()` and `Profile::line_ending()` accessors for
  direct wrapped-profile policy inspection.
- Added `Display` for `Engine` and `Profile` so padding and wrapping policy can
  be logged without relying on verbose debug output.
- Added matching `Display` output for `ct::CtEngine` so sensitive-path decoder
  policy logging uses the same dependency-free formatting surface.
- Added `Engine::profile()` for explicit dependency-free promotion to an
  unwrapped `Profile`.
- Added `ct::CtEngine::decoded_len()` so sensitive decode paths can size
  caller-owned buffers without switching to the diagnostic decoder.
- Added `Engine::ct_decoder()` for explicit promotion to the matching
  constant-time-oriented decoder without type annotations.
- Added isolated dudect, fuzz, and performance harness compile/dependency
  checks to the standard local gate so harness policy is verified before
  release-only evidence steps.
- Aligned SBOM, fuzzing, dependency, and release-evidence wording with the
  standard local gate now checking isolated harness dependencies.
- Added checked stream adapter `try_into_inner()` helpers that recover the
  wrapped reader or writer only when doing so will not discard pending input or
  buffered output.
- Added matching stream adapter `can_into_inner()` readiness helpers for
  non-consuming recovery checks in framed protocols and diagnostics.
- Added stream adapter `pending_input_needed_len()` helpers so callers can see
  how many more bytes are needed to complete a buffered encode or decode
  quantum.
- Added reader-side stream `buffered_output_capacity()` and
  `buffered_output_remaining_capacity()` helpers for fixed-queue diagnostics.
- Expanded redacted stream `Debug` output with recovery readiness, pending
  quantum state, and reader-side fixed output queue capacity.
- Expanded the isolated `stream_chunks` fuzz target to cover encoder-reader
  streaming and stream state-helper invariants.
- Added writer-side stream output queues with buffered-output diagnostics so
  wrapped writer failures can be retried without re-encoding or re-decoding
  accepted input.
- Documented and tested direct writer-adapter `write()` partial-progress
  behavior; `write_all()` remains the recommended whole-slice path.
- Changed writer-side stream output draining to write queued data in bounded
  chunks while discarding only bytes accepted by the wrapped writer, with
  short-write regressions for encoder and decoder adapters.
- Changed reader-side stream output draining to copy queued data into caller
  buffers in bounded slices while clearing queue slots as bytes are consumed.
- Hardened stream decoders to fail closed after malformed Base64 input, with
  `is_failed()` state inspection and regressions for writer and reader adapters.
- Extended the stream fuzz harness to assert fail-closed decoder state
  invariants after malformed input.
- Documented stream decoder failed-state behavior in crate rustdoc and the
  migration guide.
- Expanded the no-alloc portability smoke crate to cover scalar and
  constant-time clear-tail cleanup APIs on success and error paths.
- Expanded the no-alloc portability smoke crate to cover named MIME, PEM,
  bcrypt, and crypt profiles without enabling alloc or std.
- Hardened release metadata validation so required no-alloc smoke coverage
  symbols are checked before packaging.
- Added zero-dependency `FromStr` interop for `DecodedBuffer` and
  `SecretBuffer`, using the existing strict standard padded decode policy.
- Added zero-dependency `TryFrom<&[u8; N]>` interop for `EncodedBuffer`,
  `DecodedBuffer`, and `SecretBuffer` so byte-string literals use the same
  explicit strict standard policy as byte slices.
- Expanded no-alloc smoke coverage and release metadata validation for the
  native byte-array and `FromStr` buffer interop surfaces.
- Refreshed release evidence and checklist wording for native interop smoke
  coverage and the installed Linux, FreeBSD, wasm32, ARM, and Cortex-M target
  set.
- Hardened release metadata validation so the stable release gate must keep
  invoking the release-only evidence scripts for Miri, fuzz, cross-targets,
  backend evidence, Kani, assembly evidence, SBOMs, and reproducibility.
- Hardened release metadata validation for the trust dashboard's
  zero-dependency, scalar-only, constant-time non-claim, hardware-acceleration
  non-claim, and deferred ecosystem-integration wording.
- Hardened release metadata validation for the README's zero-dependency,
  scalar-only development, inert future-feature, constant-time non-claim, and
  release-evidence wording.
- Added no-default-features doctests to the standard local gate and release
  evidence docs so no-alloc examples are checked alongside all-features
  doctests.
- Added no-default-features documentation builds to the standard local gate and
  release evidence docs so the `no_std` API reference is built alongside the
  all-features docs.
- Added dependency-free `Engine` convenience constructors for `std::io` stream
  encoder/decoder writer and reader adapters.
- Updated the roadmap to make `v0.10` an audit-preparation milestone, add
  `v0.11` verification hardening, add `v0.12` stabilization rehearsal, and keep
  `v1.0` gated on evidence instead of schedule.
- Documented the high-assurance stack-frame cleanup boundary and added focused
  `decode_chunk` bit-packing verification to the pre-`v1.0` roadmap.

## 0.8.0 - 2026-05-16

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
  hatch where redaction and `SecretBuffer` drop-time cleanup intentionally end.
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

## 0.7.0 - 2026-05-15

- Started the next development cycle after the `0.6.0` release.
- Scoped `0.7.0` as a scalar-only security-evidence release; active SIMD
  dispatch remains intentionally out of scope until a later admission milestone.
- Added a release-gated SIMD admission validator that keeps active dispatch
  scalar-only until accelerated backend evidence is updated deliberately.
- Added wasm `simd128` candidate reporting and a reserved `no_std`
  feature-bundle compile check while keeping scalar as the only active backend.
- Added SSSE3/SSE4.1 candidate reporting and reserved feature-bundle compile
  evidence for older x86 CPUs before any active SIMD admission.
- Added an inactive SSSE3/SSE4.1 fixed-block encode prototype with
  scalar-equivalence tests while keeping runtime dispatch scalar-only.
- Added reserved SIMD feature-bundle compile checks to the normal local
  `scripts/checks.sh` gate so day-to-day checks match release expectations.
- Added an isolated dependency-free dudect-style timing harness for the scalar
  constant-time-oriented decoder, with compile/dependency checks in CI and
  opt-in local timing runs.
- Added release assembly evidence generation for no-default-features and
  all-features constant-time generated-code review.
- Added rustc metadata, review-focus notes, and artifact checksums to the
  generated constant-time assembly evidence.
- Added bounded Kani proof harnesses for constant-time-oriented decode result
  bounds, error cleanup, and validate/decode agreement.
- Added manifest generation for opt-in dudect-style timing evidence, including
  toolchain metadata, command parameters, raw output checksum, and result status.
- Added deterministic regression coverage that constant-time-oriented
  validation and decode agree on valid and malformed inputs across supported
  alphabets and padding modes.
- Hardened streaming adapters to wipe short-lived stack buffers used for
  temporary encoded, decoded, and read staging data.
- Documented the throughput tradeoff of the conservative custom alphabet
  encoding fallback.

## 0.6.0 - 2026-05-14

- Started the next development cycle after the `0.5.0` release.
- Added no-alloc validation-only APIs for strict and legacy profiles:
  `validate_result`, `validate`, `validate_legacy_result`, and
  `validate_legacy`.
- Added constant-time-oriented validation-only APIs:
  `ct::CtEngine::validate_result` and `ct::CtEngine::validate`.
- Added dependency-free line-wrapped encoding with `LineWrap`, `LineEnding`,
  checked wrapped-length calculation, caller-owned output APIs, clear-tail
  wrapping, and `alloc` convenience helpers.
- Added strict line-wrapped validation and decoding APIs that accept only the
  configured line ending and enforce configured-width non-final lines.
- Added custom alphabet helpers: `validate_alphabet`,
  `decode_alphabet_byte`, and `AlphabetError`.
- Added named dependency-free profiles for MIME, PEM, bcrypt-style, and
  `crypt(3)`-style Base64 through `Profile`, `MIME`, `PEM`, `PEM_CRLF`,
  `BCRYPT`, and `CRYPT`.
- Added `EncodedBuffer` and `encode_buffer` helpers for stack-backed short
  encoded output without requiring `alloc`.
- Added `SecretBuffer`, `encode_secret`, and `decode_secret` helpers for
  redacted owned sensitive output with dependency-free best-effort cleanup.
- Added `docs/TRUST.md`, `docs/SECURITY_CONTROLS.md`, and a README trust
  dashboard for adoption-focused security evidence and CWE mapping.
- Added `docs/PANIC_POLICY.md` and `scripts/validate-panic-policy.sh` to keep
  runtime panic-like sites reviewed and release-gated.
- Added `scripts/check_miri.sh` and routed CI/release Miri checks through it so
  both no-default scalar and all-features alloc/stream surfaces run under Miri
  when nightly Miri is installed.
- Added `docs/FUZZING.md` and `scripts/check_fuzz_corpus.sh` to document and
  enforce reviewed fuzz corpus handling.
- Expanded `docs/CONSTANT_TIME.md` with generated-code review requirements and
  added `scripts/validate-constant-time-policy.sh` to release-gate the current
  constant-time non-claim wording.
- Expanded gated Kani proof harness definitions for slice encode/decode,
  clear-tail decode, and in-place encode bounds while keeping execution gated
  on Kani's bundled compiler support.
- Hardened scalar chunk validation and decode helpers to use checked quad
  reads and typed `[u8; 4]` inputs instead of debug-asserted slice lengths.
- Replaced the cleanup helper's ordinary zero fill with an audited volatile
  write loop so best-effort wiping is not optimized away.
- Reduced constant-time-oriented padded terminal handling by replacing explicit
  padding-count branches with masked final-quantum validation and
  length-derived final writes.

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
