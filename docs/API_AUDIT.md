# Public API Audit

This document records the public API audit that prepared the `v1.0` candidate
series. The goal is a small, explicit, security-reviewed public surface. The
audit favors removing or documenting ambiguous APIs over adding convenience.

## Audit Rules

- Keep strict, canonical decoding as the default.
- Keep legacy, wrapped, profile-specific, and constant-time-oriented behavior
  explicit in names or modules.
- Do not add broad conversion traits that hide alphabet, padding, wrapping,
  allocation, or secret-handling policy.
- Do not admit optional ecosystem dependencies without a written dependency
  admission record.
- Do not admit active SIMD dispatch through an API audit; SIMD remains governed
  by the dedicated SIMD admission manifest.
- Treat any API that exposes owned bytes or backing arrays as a security
  boundary that must be documented.
- Prefer caller-owned output APIs and recoverable errors for untrusted input.

## Status Legend

- `candidate stable`: the current API shape is accepted for the `v1.0`
  candidate, pending final CI and pentest evidence.
- `documented boundary`: the API is acceptable only because its security or
  ownership boundary is explicit in docs and examples.
- `review pending`: the API still needs audit before a stable release; stable
  releases fail the local gate if any row remains in this state.
- `deferred`: the API area is intentionally not admitted in this release
  series.

## Public Surface Under Review

| Area | Status | Notes |
| --- | --- | --- |
| Engine constants and `Engine<A, PAD>` | candidate stable | Strict/default semantics are explicit; constructor naming and stream convenience methods are accepted for the `v1.0` candidate. |
| `Profile<A, PAD>` and named profiles | candidate stable | MIME, PEM, bcrypt-style, `crypt(3)`-style, wrapping, and padding behavior are explicit and covered by policy tests. |
| Length helpers | candidate stable | Public helpers are recoverable and checked; keep examples focused on untrusted-size handling. |
| Slice encode/decode APIs | candidate stable | Caller-owned output, checked lengths, and clear-tail variants are the preferred stable surface. |
| In-place APIs | candidate stable | Encode-to-back and decode-to-front contracts are explicit, checked, and paired with clear-tail variants. |
| Validation-only APIs | candidate stable | Strict, legacy, wrapped, and ct validation APIs are documented as decode-equivalent policy checks. |
| Stack-backed buffers | documented boundary | `EncodedBuffer` and `DecodedBuffer` are retained with explicit visible-length, cleanup, comparison, and exposed-array boundaries. |
| `SecretBuffer` | documented boundary | Redaction, cleanup limits, comparison semantics, and owned escape hatches are explicit adoption boundaries. |
| `ct` module | documented boundary | Keep non-claim wording and opaque error behavior explicit unless verification evidence changes. |
| `stream` module | documented boundary | Fail-closed decode, retry semantics, state helpers, recovery helpers, and framed-reader behavior are explicit. |
| Runtime backend reporting | candidate stable | Scalar-only posture and stable log identifiers are documented and release-gated. |
| Feature flags | documented boundary | `tokio`, `kani`, `fuzzing`, and `simd` remain inert or reserved unless admitted by their policy docs. `allow-wasm32-best-effort-wipe` is an explicit opt-in for wasm builds that accept compiler-fence-only cleanup; `allow-compiler-fence-only-wipe` is the equivalent opt-in for unsupported native architectures. AArch64 CSDB attestation uses the custom cfg `base64_ng_aarch64_csdb_attested`, not a Cargo feature, so `--all-features` cannot enable it accidentally. High-assurance deployments may use custom cfg `base64_ng_require_high_assurance` to fail builds that enable `simd`. |
| Error types | candidate stable | Encode, decode, and alphabet errors are recoverable and diagnostic without committing ct errors to localized detail. |
| Macros and custom alphabets | documented boundary | Compile-time validation and conservative fixed-scan performance/security tradeoffs are explicit. |

## `v1.0` Admission Questions

- Is the API name explicit about strict, legacy, wrapped, profile, or
  constant-time-oriented behavior?
- Can the caller size memory before using the API?
- Does the API have a caller-owned buffer form when it can allocate?
- Does the API expose a clear cleanup boundary for sensitive data?
- Does the API keep malformed input recoverable instead of panicking?
- Does the API commit to behavior that future SIMD backends can reproduce
  exactly?
- Does the API require dependency admission or feature-policy documentation?

## Audit Decisions

### Profiles

`Profile<A, PAD>` and the named `MIME`, `PEM`, `PEM_CRLF`, `BCRYPT`, and
`CRYPT` profiles are candidates for the `v1.0` stable surface.

Decision rationale:

- A profile is an explicit policy bundle: alphabet, padding mode, and optional
  line wrapping remain visible through type parameters, constructor arguments,
  and policy accessors.
- `Profile::checked_new` rejects invalid wrapping policy instead of silently
  accepting an unusable profile.
- MIME and PEM wrapping policy is strict: non-final encoded lines must match
  the configured width and line ending.
- Bcrypt-style and `crypt(3)`-style profiles expose alphabet and no-padding
  interoperability only; they do not claim password-hash parsing or
  verification.
- Profiles forward to the same scalar engine, validation, in-place, clear-tail,
  stack-buffer, and secret-buffer APIs rather than introducing a separate
  decoding contract.

Stable boundary:

- Keep profile behavior strict and deterministic.
- Do not add permissive profile constructors without a new audit entry.
- Do not broaden bcrypt-style or `crypt(3)`-style profiles into full password
  hash parsers.
- Do not hide profile policy behind broad conversion traits.

### Validation-Only APIs

Strict, legacy, wrapped, and constant-time-oriented validation APIs are
candidates for the `v1.0` stable surface.

Decision rationale:

- Validation-only APIs use the same alphabet, padding, canonical-bit, and line
  wrapping checks as the corresponding decode APIs.
- Boolean helpers are convenience wrappers over `Result`-returning helpers,
  preserving recoverable diagnostics for callers that need them.
- Legacy validation is opt-in and only skips ASCII transport whitespace; it
  keeps alphabet, padding, terminal-data, and canonical-bit checks strict.
- Wrapped validation is stricter than legacy whitespace handling and accepts
  only the configured wrapping policy.
- Constant-time-oriented validation follows the `ct` module's documented
  opaque malformed-input policy.

Stable boundary:

- Keep validation/decode agreement release-tested.
- Keep strict validation canonical by default.
- Keep legacy and wrapped validation explicit in method names.
- Keep ct validation errors opaque unless formal side-channel evidence changes
  the documented contract.

### Stack-Backed Buffers

`EncodedBuffer<CAP>` and `DecodedBuffer<CAP>` are retained as documented
boundaries for `v1.0`.

Decision rationale:

- Stack-backed buffers give no-alloc callers an owned output shape without
  hiding capacity or visible length.
- Accessors expose borrowed bytes or fallible UTF-8 views rather than
  implicitly allocating or assuming decoded text.
- `Debug` is redacted for decoded buffers; encoded buffers remain printable as
  Base64 text through explicit display/text APIs.
- Drop-time cleanup is best-effort and scoped to the buffer's current backing
  array, not historical stack-frame copies.
- `EncodedBuffer<CAP>` and `DecodedBuffer<CAP>` rely on Rust auto traits for
  `Send` and `Sync`; no explicit `unsafe impl` is used. If a future internal
  field is not thread-safe, auto-trait derivation should fail instead of being
  overridden by a stale manual implementation.
- `into_exposed_array` is intentionally named as an ownership escape hatch and
  returns `ExposedEncodedArray` or `ExposedDecodedArray`, preserving redaction
  and drop-time cleanup after ownership transfer. The raw-array escape hatch is
  deliberately loud:
  `into_exposed_unprotected_array_caller_must_zeroize`.
- Equality is intentionally not exposed through `PartialEq`/`==`. Callers must
  opt into the explicit `constant_time_eq_public_len` helper, whose equal-length
  scan is best-effort and whose length mismatch remains public.

Stable boundary:

- Keep capacity and visible length explicit.
- Keep ownership escape hatches explicit in names.
- Do not add implicit text conversions for decoded bytes.
- Do not describe drop-time cleanup as formal zeroization.

### Secret Buffer

`SecretBuffer` is retained as a documented security boundary for `v1.0`.

Decision rationale:

- Formatting is redacted by default through `Debug` and `Display`.
- Secret exposure requires explicitly named borrowed or owned escape hatches.
- `SecretBuffer::into_exposed_vec` returns `ExposedSecretVec`, which keeps
  redacted formatting and drop-time cleanup. The raw-vector escape hatch is the
  deliberately loud
  `ExposedSecretVec::into_exposed_unprotected_vec_caller_must_zeroize`.
- `SecretBuffer::try_into_exposed_string` returns `ExposedSecretString`, which
  keeps redacted formatting and drop-time cleanup. The raw-string escape hatch
  is the deliberately loud
  `ExposedSecretString::into_exposed_unprotected_string_caller_must_zeroize`.
- Drop-time cleanup uses the crate's volatile best-effort wipe helper for
  initialized bytes and vector spare capacity.
- `SecretBuffer` does not lock allocations into physical memory. OS paging,
  hibernation, and crash-dump controls remain deployment responsibilities.
- `SecretBuffer` intentionally does not implement `Clone`; callers that need a
  second copy must spell that out through `from_slice`, making secret
  duplication visible at the call site.
- `SecretBuffer` relies on Rust auto traits for `Send` and `Sync`; no explicit
  `unsafe impl` is used. This is deliberate because a future non-thread-safe
  field should remove those auto traits automatically.
- Equality is intentionally not exposed through `PartialEq`/`==`. Callers must
  opt into the explicit `constant_time_eq_public_len` helper, whose equal-length
  scan is best-effort and whose length mismatch remains public.
- Strict standard padded `TryFrom` and `FromStr` implementations are kept only
  for native Rust ergonomics; non-standard profiles remain on explicit
  engine/profile methods.

Stable boundary:

- Keep redaction as the default formatting behavior.
- Keep `expose_secret`, `into_exposed_vec`,
  `into_exposed_unprotected_vec_caller_must_zeroize`, and
  `try_into_exposed_string`,
  `into_exposed_unprotected_string_caller_must_zeroize` explicit.
- Do not claim formal zeroization or allocator-wide cleanup.
- Do not add broad conversions that hide profile, alphabet, padding, or
  wrapping policy.

### In-Place APIs

In-place encode and decode APIs are candidates for the `v1.0` stable surface.

Decision rationale:

- In-place encode validates the caller-provided input length and required
  encoded length before writing.
- In-place encode writes from the back of the output region toward the front,
  so unread input bytes are not overwritten.
- In-place decode writes decoded output to the front of the same buffer, which
  is valid because Base64 decoded output is never larger than accepted encoded
  input.
- Legacy and wrapped in-place decode validate and compact input according to
  their explicit policies before decoding.
- Clear-tail variants exist for strict, legacy, wrapped, and
  constant-time-oriented in-place decode when the caller wants best-effort
  cleanup on success and failure.

Stable boundary:

- Keep all in-place APIs recoverable through `Result`.
- Keep strict, legacy, wrapped, and ct in-place behavior separated by method or
  module name.
- Keep clear-tail variants explicit rather than making cleanup an implicit
  default for all in-place APIs.
- Keep the removed non-clear-tail `ct::CtEngine::decode_slice` and
  `ct::CtEngine::decode_in_place` APIs out of the stable surface. They could
  leave decoded plaintext in caller-owned buffers on malformed input errors.
  Direct reusable secret buffers to `ct::CtEngine::decode_slice_clear_tail`,
  `ct::CtEngine::decode_buffer`, or
  `ct::CtEngine::decode_in_place_clear_tail`.
- Do not add unchecked in-place APIs to the public surface.

### Custom Alphabets

`validate_alphabet`, `decode_alphabet_byte`, and `define_alphabet!` are retained
as documented boundaries for `v1.0`.

Decision rationale:

- Custom alphabets must contain exactly 64 unique visible ASCII bytes and must
  not contain the padding byte.
- `define_alphabet!` validates the alphabet at compile time, so invalid
  literals fail the build.
- The generated `decode` method delegates to the same validated table
  semantics as runtime custom alphabet helpers.
- The default `Alphabet::encode` implementation performs a fixed 64-entry scan
  for every emitted byte. This preserves the conservative no secret-indexed
  lookup posture, but it is slower than the arithmetic mappers used by built-in
  alphabets.
- Manual `Alphabet` implementations can override `encode` or `decode`; those
  overrides affect the normal `Engine` path. The `ct` module scans
  `Alphabet::ENCODE` directly and does not depend on custom `decode`
  implementations.

Stable boundary:

- Keep compile-time validation in the macro.
- Keep custom-alphabet performance tradeoffs documented.
- Keep the custom-alphabet timing contract documented on the `Alphabet` trait
  and in the macro docs.
- Do not add a faster custom-alphabet path unless it has its own audit record.
- Do not accept non-visible ASCII or padding bytes in Base64 alphabets.

### Stream Module

The `stream` module is retained as a documented boundary for `v1.0`.

Decision rationale:

- Streaming remains behind the explicit `stream` feature and depends only on
  `std::io`.
- Writer adapters expose `try_finish` for finalization without consuming the
  adapter, and `finish` for finalization plus wrapped object recovery.
- Writer adapters buffer accepted output and allow failed wrapped writes to be
  retried without re-encoding or re-decoding already accepted input.
- Direct `Write::write` calls follow normal `std::io::Write` partial-progress
  semantics; examples and migration docs recommend `write_all` when the whole
  slice must be consumed.
- Decoder adapters fail closed after malformed Base64 input and expose
  `is_failed` for diagnostics.
- `can_into_inner` and `try_into_inner` provide checked recovery paths that
  refuse to silently discard pending input or buffered output.
- Padded `DecoderReader` stops after terminal padding and leaves adjacent
  framed payload bytes unread in the wrapped reader.
- Debug output redacts wrapped I/O values and pending payload bytes while still
  exposing non-sensitive state useful for diagnostics.
- Internal pending and output queues are wiped on consumption and drop as
  best-effort retention reduction.

Stable boundary:

- Keep `std::io` streaming under the explicit `stream` feature.
- Keep async/Tokio out unless the async admission policy is satisfied.
- Keep decoder failure fail-closed.
- Keep recovery helpers explicit; do not make unchecked `into_inner` the
  recommended safe recovery path.
- Keep reader terminal-padding behavior documented for framed protocols.

### Error Types

`EncodeError`, `DecodeError`, and `AlphabetError` are candidates for the
`v1.0` stable surface.

Decision rationale:

- Public runtime errors are recoverable through `Result` and avoid panic-based
  failure for malformed input, size errors, and invalid policies.
- `EncodeError` separates length overflow, invalid line wrapping, input length,
  output capacity failures, invalid alphabet output, and accelerated-backend
  scalar mismatch failures.
- `DecodeError` separates invalid length, invalid bytes, invalid padding,
  invalid line wrapping, output capacity, and deliberately opaque malformed
  input.
- Strict, legacy, wrapped, and in-place decode paths preserve absolute input
  indexes where localized diagnostics are part of the public contract.
- `ct` APIs intentionally report malformed content as `InvalidInput` so the
  constant-time-oriented path does not promise localized error detail.
- `AlphabetError` identifies invalid, padding, and duplicate alphabet bytes
  during custom alphabet validation.

Stable boundary:

- Keep existing variants unless a later release-candidate audit finds a
  correctness reason to change them during the `v1.0` candidate review.
- Keep localized indexes for non-ct scalar diagnostics.
- Keep ct malformed-content errors opaque.
- Do not add panicking convenience APIs for error cases already represented by
  public error variants.

## Initial `v0.10` Direction

- Keep async/Tokio, serde, bytes, zeroize, subtle, property-test, and
  Criterion-style integrations out unless a concrete admission record is
  written.
- Keep `ct` documented as constant-time-oriented rather than formally
  cryptographic unless verification evidence improves during the `v0.10` to
  `v0.12` candidate series.
- Keep active dispatch scalar-only until SIMD admission evidence is complete.
- Focus implementation work on audit findings, documentation gaps, tests, and
  evidence rather than new feature breadth.

## `v1.0` Candidate Outcome

- No public API area remains marked `review pending`.
- Deferred ecosystem integrations remain outside the stable contract until
  they pass dependency admission.
- The `ct` module remains constant-time-oriented and does not claim formal
  cryptographic constant-time behavior.
- Active backend dispatch remained scalar-only at the `v1.0` boundary; later
  admitted SIMD backends remain governed by the SIMD admission policy and the
  trust dashboard.
- A future secure-decode marker trait or wrapper type remains a post-`v1.0`
  candidate. It should be designed only after the stable `ct` contract is
  exercised by downstream users, so the `v1.0` boundary relies on explicit
  `ct` constants, `Engine::ct_decoder()`, and prominent default-decoder
  warnings rather than a late broad API addition.

## Current Post-`1.3.3` Outcome

- Public encode and normal strict decode have admitted SIMD acceleration only
  for the Standard and URL-safe alphabet families under the runtime profiles
  documented in `docs/SIMD_ADMISSION.md`.
- The narrow wasm `simd128` profile is admitted only for binaries compiled with
  `target-feature=+simd128`, `simd`, and
  `allow-wasm32-best-effort-wipe`, backed by the runtime smoke evidence named
  in `docs/WASM_SIMD128_RUNTIME_REVIEW.md`.
- Non-standard surfaces remain intentionally scalar unless a later admission
  package covers them: custom alphabets, bcrypt-style and `crypt(3)` profiles,
  `ct` secret decode, broader wasm/browser claims, big-endian AArch64, and
  `no_std` dispatch. Wrapped, legacy, and strict in-place decode are admitted
  only after scalar validation and staging; line-profile validation,
  line-ending compaction, and legacy-whitespace compaction remain scalar.
