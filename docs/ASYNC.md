# Async Admission Policy

`base64-ng` does not currently provide async streaming wrappers in the core
crate. The core `tokio` feature is intentionally inert and dependency-free.
Async integration lives in the optional `base64-ng-tokio` companion crate so
the core package remains `no_std`-first and dependency-free by default.

The optional `base64-ng-tokio` companion crate is admitted separately for
read-all/write-all helper functions and manual `AsyncRead`/`AsyncWrite`
streaming adapters. Its `*_limited` helpers enforce a caller-provided maximum
input size before writing output. Its `EncoderReader`, `DecoderReader`,
`EncoderWriter`, and `DecoderWriter` adapters use explicit state machines,
fixed internal buffers, and drop cleanup.

Read-all helper allocations are held behind RAII guards before the first
suspension point. Their initialized bytes and spare capacity are wiped on
success, I/O error, or future cancellation. Limited helpers request at most the
remaining allowance plus one lookahead byte. Generic `AsyncRead` cannot return
that lookahead byte to the source; callers that must preserve adjacent framed
input should provide an already bounded reader or use a streaming adapter. The
limited helpers cap eager allocation at 8 KiB and wipe only the bytes filled by
each successful read; their RAII guards still wipe complete live allocations
and the complete staging array on cancellation or drop. When a read-all vector
must grow, the helper copies into a guarded replacement and wipes the previous
allocation before deallocation so historical growth buffers are not released
with live frame contents.

## Current Status

- The `stream` feature provides `std::io` streaming wrappers.
- The `tokio` feature is reserved and currently expands to an empty feature set.
- `scripts/check_reserved_features.sh` verifies that `tokio` remains inert and
  dependency-free until admission.
- No async traits, Tokio types, or async runtime dependencies are exported by
  the crate today.
- `base64-ng-tokio` provides optional read-all/write-all helpers for projects
  that already admit Tokio. Prefer its limited helpers for peer-controlled
  input. Their temporary allocations use cancellation-safe RAII cleanup.
- `base64-ng-tokio` also provides streaming adapters: `EncoderReader`,
  `DecoderReader`, `EncoderWriter`, and `DecoderWriter`.
- Async writer shutdown is the finalization boundary. Call
  `AsyncWriteExt::shutdown` to encode or validate final partial quanta before
  recovering the wrapped writer.

## Admission Requirements

Before the core `tokio` feature may add a dependency or public API, or before
`base64-ng-tokio` admits a new async state-machine surface, the change must
include:

- A written dependency review covering the Tokio version, transitive
  dependency graph, licenses, advisories, and why `std` is insufficient.
- `tokio` must stay optional and must not become a default feature.
- The non-async `stream` API must remain available without Tokio.
- Cancellation behavior must be specified for partially buffered plaintext,
  encoded output, pending decode input, and terminal padding states.
- Drop behavior must clear internal staging buffers with the same best-effort
  retention-reduction posture as the current `std::io` wrappers.
- Chunk-boundary tests must cover every admitted direction split at Base64
  quantum boundaries.
- Adjacent framed payload tests must prove decoder readers do not consume bytes
  beyond terminal padding.
- Fuzz or adversarial polling coverage must include fragmented async-like chunk
  schedules before any performance claim is made.
- Release evidence must include `cargo deny check`, `cargo audit`, and
  `cargo license --json` output with the async feature enabled.

## Non-Goals

- Async wrappers are not a reason to weaken strict Base64 validation.
- Async wrappers must not enable SIMD dispatch or unsafe code by default.
- Async wrappers must not become the primary API; caller-owned buffers and
  scalar strict semantics remain the reference behavior.

## Release Rule

Do not advertise a new async/Tokio surface in release notes until it exports a
tested public API and the dependency/admission evidence is present. Reader
streaming, writer streaming, and read-all/write-all helpers are admitted in the
companion crate.
