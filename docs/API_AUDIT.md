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
| `Profile<A, PAD>` and named profiles | review pending | Freeze MIME, PEM, bcrypt-style, `crypt(3)`-style, wrapping, and padding behavior. |
| Length helpers | candidate stable | Public helpers are recoverable and checked; keep examples focused on untrusted-size handling. |
| Slice encode/decode APIs | candidate stable | Caller-owned output, checked lengths, and clear-tail variants are the preferred stable surface. |
| In-place APIs | review pending | Confirm decode-to-front and encode-to-back contracts are clear. |
| Validation-only APIs | review pending | Confirm validate/decode agreement across strict, legacy, wrapped, and ct paths. |
| Stack-backed buffers | review pending | Confirm exposed-array escape hatches and cleanup boundaries are documented. |
| `SecretBuffer` | review pending | Confirm redaction, cleanup limits, comparison semantics, and ownership escape hatches. |
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
