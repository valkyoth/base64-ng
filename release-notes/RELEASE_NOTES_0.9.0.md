# base64-ng 0.9.0 Release Notes

Status: released

## Summary

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

## Commit Range

- Previous tag: `v0.8.0`
- Release tag: `v0.9.0`
- Release date: `2026-05-17`

## Commits

### Added

- `b90d130` Add stream state inspection helpers
- `f3f2fd0` Add no-alloc portability smoke gate
- `7277c8d` Add redacted stream diagnostics
- `6d71939` Add fragmented stream boundary regressions
- `acfeeee` Expand no-alloc smoke API coverage
- `f2b8ea9` Add non-consuming stream finish helpers
- `f804f23` Expand no-alloc smoke coverage
- `5fff55e` Add display for encoded buffers
- `d3cad90` Add fallible encoded buffer text view
- `0977ad3` Add line ending text accessor
- `edf059d` Add line wrap policy accessors
- `424b9b1` Add printable line ending identifiers
- `9e02908` Add printable line wrap formatting
- `a14f7e6` Add profile wrapping policy accessors
- `db1533f` Add printable engine and profile policy output
- `37a0586` Add printable constant-time engine policy output
- `4c334a7` Add constant-time decoded length helper
- `c016874` Add checked stream inner recovery
- `edcf326` Add stream inner recovery readiness checks
- `57bf82c` Add stream pending quantum length helpers
- `d7c75aa` added CODEOWNERS file
- `153171c` Expand redacted stream debug state
- `1fb7c65` Add isolated harnesses to standard checks
- `7f63b0b` Expand no-alloc clear-tail smoke coverage
- `fc79c79` Add strict FromStr decode interop
- `e112841` Add byte-array buffer conversions
- `3c54cde` Add engine stream constructors

### Security / Hardening

- `f084428` Harden constant-time evidence gates
- `4393ad1` Harden stream finalization state
- `06d1bf3` Gate README security posture claims
- `73d5b03` Document pentest hardening follow-ups

### Documentation

- `c31c556` Document deferred dependency integrations
- `b525470` Document stream state migration guidance
- `902afbe` Document stream retry semantics
- `d55248f` Document stream writer partial writes
- `5d5c400` Document stream decoder failure state
- `c1336ac` Gate release evidence wording
- `32b1841` Gate release-only evidence scripts
- `57de485` Build no-default docs in local gate
- `349179b` Prepare 0.9.0 release
- `e2faeee` Extend pre-1.0 release roadmap

### Verification

- `a1fc4d0` Run no-alloc smoke in target CI
- `f9e66c7` Run no-alloc smoke tests on host
- `8ac958f` Test stream finalization retry behavior
- `025c11f` Test stream finish flush retry behavior
- `d02f883` Test stream decoder flush retry behavior
- `06f6214` Test stream decoder finish write retry behavior
- `5d1a918` Test stream write retry behavior
- `8c37a75` Add explicit engine profile helper
- `2bf455e` Add explicit constant-time decoder helper
- `4de63bc` Add stream buffered output capacity helpers
- `44b426e` Expand stream fuzz state coverage
- `7475e2f` Align isolated harness evidence wording
- `d3d792c` Assert stream decoder failure in fuzzing
- `e584cab` Run no-default doctests in local gate

### Other Changes

- `3aabb0e` Start 0.9 development cycle
- `601282f` Expose stream reader buffered output state
- `e836287` Enforce deferred integration features
- `6a1fb69` Align no-alloc smoke target defaults
- `d5f1cf2` Expose stream finalization state
- `59470ac` Expose stream policy accessors
- `8ca8963` Expose stream reader input completion state
- `3266573` Buffer stream writer output for retry
- `396c4f6` Drain stream writer output in chunks
- `5b7dddd` Drain stream reader output in slices
- `2534579` Fail closed on malformed stream decode
- `303e0eb` Cover named profiles in no-alloc smoke
- `7c5e889` Gate no-alloc smoke coverage metadata
- `aa3232a` Cover native interop in no-alloc smoke
- `20a53b9` Gate trust dashboard claims

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.9.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
