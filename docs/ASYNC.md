# Async Admission Policy

`base64-ng` does not currently provide async streaming wrappers. The `tokio`
feature is intentionally inert and dependency-free until an async API is
admitted through the same evidence-driven process used for SIMD and other
security-sensitive surfaces.

This keeps the published crate small and avoids adding a runtime dependency
before the API, cancellation behavior, and security tradeoffs are documented.

## Current Status

- The `stream` feature provides `std::io` streaming wrappers.
- The `tokio` feature is reserved and currently expands to an empty feature set.
- `scripts/check_reserved_features.sh` verifies that `tokio` remains inert and
  dependency-free until admission.
- No async traits, Tokio types, or async runtime dependencies are exported by
  the crate today.

## Admission Requirements

Before the `tokio` feature may add a dependency or public API, the change must
include:

- A written dependency review covering the Tokio version, transitive
  dependency graph, licenses, advisories, and why `std` is insufficient.
- `tokio` must stay optional and must not become a default feature.
- The non-async `stream` API must remain available without Tokio.
- Cancellation behavior must be specified for partially buffered plaintext,
  encoded output, pending decode input, and terminal padding states.
- Drop behavior must clear internal staging buffers with the same best-effort
  retention-reduction posture as the current `std::io` wrappers.
- Chunk-boundary tests must cover reads and writes split at every Base64
  quantum boundary.
- Adjacent framed payload tests must prove decoder readers do not consume bytes
  beyond terminal padding.
- Fuzz coverage must include fragmented async-like chunk schedules before any
  performance claim is made.
- Release evidence must include `cargo deny check`, `cargo audit`, and
  `cargo license --json` output with the async feature enabled.

## Non-Goals

- Async wrappers are not a reason to weaken strict Base64 validation.
- Async wrappers must not enable SIMD dispatch or unsafe code by default.
- Async wrappers must not become the primary API; caller-owned buffers and
  scalar strict semantics remain the reference behavior.

## Release Rule

Do not advertise async or Tokio support in release notes until the feature
exports a tested public API and the dependency/admission evidence is present.
