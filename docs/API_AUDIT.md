# Public API Audit

This document tracks the `v0.10` audit-preparation milestone. The goal is to
enter the `v1.0` candidate series with a small, explicit, security-reviewed
public surface. The audit favors removing or documenting ambiguous APIs over
adding convenience.

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

- `candidate stable`: the current API shape looks suitable for `v1.0`, pending
  final release-candidate evidence.
- `documented boundary`: the API is acceptable only because its security or
  ownership boundary is explicit in docs and examples.
- `review pending`: the API still needs audit before a stable `v0.10` release;
  stable releases fail the local gate if any row remains in this state.
- `deferred`: the API area is intentionally not admitted in this release
  series.

## Public Surface Under Review

| Area | Status | Notes |
| --- | --- | --- |
| Engine constants and `Engine<A, PAD>` | candidate stable | Strict/default semantics are explicit; audit constructor naming and stream convenience methods before `v1.0`. |
| `Profile<A, PAD>` and named profiles | candidate stable | MIME, PEM, bcrypt-style, `crypt(3)`-style, wrapping, and padding behavior are explicit and covered by policy tests. |
| Length helpers | candidate stable | Public helpers are recoverable and checked; keep examples focused on untrusted-size handling. |
| Slice encode/decode APIs | candidate stable | Caller-owned output, checked lengths, and clear-tail variants are the preferred stable surface. |
| In-place APIs | review pending | Confirm decode-to-front and encode-to-back contracts are clear. |
| Validation-only APIs | candidate stable | Strict, legacy, wrapped, and ct validation APIs are documented as decode-equivalent policy checks. |
| Stack-backed buffers | documented boundary | `EncodedBuffer` and `DecodedBuffer` are retained with explicit visible-length, cleanup, comparison, and exposed-array boundaries. |
| `SecretBuffer` | documented boundary | Redaction, cleanup limits, comparison semantics, and owned escape hatches are explicit adoption boundaries. |
| `ct` module | documented boundary | Keep non-claim wording and opaque error behavior explicit unless verification evidence changes. |
| `stream` module | review pending | Confirm fail-closed behavior, retry semantics, state helpers, and recovery helpers. |
| Runtime backend reporting | candidate stable | Scalar-only posture and stable log identifiers are documented and release-gated. |
| Feature flags | deferred | `tokio`, `kani`, `fuzzing`, and `simd` remain inert or reserved unless admitted by their policy docs. |
| Error types | review pending | Confirm variants and indexes are stable enough for downstream diagnostics. |
| Macros and custom alphabets | review pending | Confirm compile-time validation and custom-alphabet performance/security tradeoffs. |

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
- `into_exposed_array` is intentionally named as an ownership escape hatch
  where redaction and drop-time cleanup stop applying to the returned array.
- Equality uses the same constant-time-oriented equal-length comparison helper
  used by the redacted owned wrapper.

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
- Drop-time cleanup uses the crate's volatile best-effort wipe helper for
  initialized bytes and vector spare capacity.
- Equality and direct byte/text comparisons use constant-time-oriented
  equal-length comparison semantics.
- Strict standard padded `TryFrom` and `FromStr` implementations are kept only
  for native Rust ergonomics; non-standard profiles remain on explicit
  engine/profile methods.

Stable boundary:

- Keep redaction as the default formatting behavior.
- Keep `expose_secret`, `into_exposed_vec`, and `try_into_exposed_string`
  explicit.
- Do not claim formal zeroization or allocator-wide cleanup.
- Do not add broad conversions that hide profile, alphabet, padding, or
  wrapping policy.

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
