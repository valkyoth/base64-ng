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

## Public Surface Under Review

| Area | Status | Notes |
| --- | --- | --- |
| Engine constants and `Engine<A, PAD>` | review pending | Confirm strict/default semantics and constructor naming before `v1.0`. |
| `Profile<A, PAD>` and named profiles | review pending | Freeze MIME, PEM, bcrypt-style, `crypt(3)`-style, wrapping, and padding behavior. |
| Length helpers | review pending | Confirm all public helpers stay recoverable and panic-free for untrusted sizes. |
| Slice encode/decode APIs | review pending | Confirm buffer sizing, clear-tail variants, and error semantics. |
| In-place APIs | review pending | Confirm decode-to-front and encode-to-back contracts are clear. |
| Validation-only APIs | review pending | Confirm validate/decode agreement across strict, legacy, wrapped, and ct paths. |
| Stack-backed buffers | review pending | Confirm exposed-array escape hatches and cleanup boundaries are documented. |
| `SecretBuffer` | review pending | Confirm redaction, cleanup limits, comparison semantics, and ownership escape hatches. |
| `ct` module | review pending | Confirm non-claim wording and opaque error behavior remain explicit. |
| `stream` module | review pending | Confirm fail-closed behavior, retry semantics, state helpers, and recovery helpers. |
| Runtime backend reporting | review pending | Confirm scalar-only posture and stable log identifiers. |
| Feature flags | review pending | Confirm `tokio`, `kani`, `fuzzing`, and `simd` remain inert or reserved as documented. |
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
