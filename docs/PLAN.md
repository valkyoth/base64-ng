# base64-ng Professional Secure Plan

Date: 2026-05-14

## Objective

Build `base64-ng` as a modern, secure, `no_std`-first Base64 implementation for Rust. The crate must improve on the established `base64` crate only where evidence supports the claim: stricter release process, stronger verification, zero-copy ergonomics, optional streaming, and hardware acceleration that remains subordinate to proven scalar behavior.

## Current Baseline

- Rust stable: `1.95.0`.
- License: `MIT OR Apache-2.0`.
- Project name: `base64-ng`.
- Runtime and dev dependency graph: zero external crates.
- Local testing system modeled after `fluxheim`: check script, release gate, dependency policy, audit config, Miri when installed, SBOM, reproducible build check, and CI.

## Dependency Policy

The default position is no external crates.

This policy is enforced by:

```sh
scripts/validate-dependencies.sh
```

The standalone dependency admission policy lives in
[`docs/DEPENDENCIES.md`](DEPENDENCIES.md).

Allowed without adding dependencies:

- Base64 scalar logic.
- In-place decode.
- `alloc` convenience APIs.
- `std::io` streaming wrappers.
- Architecture SIMD through `core::arch`.
- Runtime CPU dispatch through the Rust standard library where available.
- Deterministic tests, table tests, and handcrafted malformed-input cases.

External crates require written justification before inclusion:

- `tokio` may be accepted only behind the optional `tokio` feature for async stream wrappers.
- `serde` may be accepted only behind an optional feature after API and dependency review.
- `bytes` may be accepted only behind an optional feature after API and dependency review.
- `zeroize` remains rejected by default; the current direction is an internal
  best-effort wiping helper and secret wrapper types before considering a
  dependency.
- Fuzzing crates may live only in a future `fuzz/` workspace or tool-specific harness, not in normal runtime dependencies.
- Benchmark crates may live only in `dev-dependencies` or an isolated bench workspace.
  The default benchmark path should remain dependency-free unless a tool such
  as Criterion earns admission with better measurement evidence.
- Kani support must stay feature-gated and must not affect normal users.

Rejected by default:

- Helper crates for simple bit manipulation, table generation, feature selection, error formatting, or CLI tooling.
- Git dependencies.
- Runtime dependencies for the default feature set.
- Dependencies with unclear licensing, advisories, yanked releases, or unnecessary transitive graphs.
- Crates that only replace small, auditable `core`/`alloc`/`std`
  implementations.

Any dependency addition must answer:

- Why can this not be implemented clearly with `core`, `alloc`, or `std`?
- Is it runtime, dev-only, fuzz-only, or CI-only?
- What transitive dependencies does it add?
- Does `cargo deny check`, `cargo audit`, and `cargo license --json` remain clean?
- Can the feature remain optional?

## Architecture

### Layers

1. `core`
   - `no_std` scalar implementation.
   - Reference semantics for all future fast paths.
   - No unsafe code.

2. `alloc`
   - `Vec<u8>` decode/encode helpers.
   - Encoded `String` helper.
   - Optional feature.

3. `simd`
   - Future AVX2, AVX-512, and ARM NEON.
   - Runtime dispatch under `std`.
   - Compile-time target-feature paths for embedded or specialized builds.
   - Unsafe isolated in `src/simd.rs` and documented.

4. `stream`
   - `std::io::{Read, Write}` wrappers.
   - Chunk-boundary state machines.

5. `tokio`
   - Reserved feature for future async wrappers.
   - Inert and dependency-free until the async API is admitted.
   - Admission requirements documented in `docs/ASYNC.md`.

### API Policy

- Stable Rust only for public APIs.
- No `generic_const_exprs` or unstable const-generic tricks in the public surface.
- Use zero-sized engines and trait-based alphabets.
- Strict decoding is default.
- Legacy behavior must be opt-in and named.
- Canonical decoding is default.
- Panicking convenience APIs are avoided in favor of checked APIs for
  untrusted sizes and untrusted input.
- New profile APIs must expose caller-owned buffer variants before adding
  allocation conveniences.

## Security Design

### Hard Rules

- Scalar encode/decode remains safe Rust.
- The scalar-side volatile wipe helpers are the only non-SIMD unsafe
  admissions.
- Unsafe SIMD must live under dedicated modules.
- `allow(unsafe_code)` must remain confined to the volatile wipe helpers and
  `src/simd.rs`.
- Every unsafe block requires a local safety explanation.
- Every SIMD path must have deterministic and fuzzed differential tests against scalar.
- Padding behavior must be canonical by default.
- Whitespace and non-alphabet bytes are rejected by default.
- Public buffer-size calculations must have checked variants for untrusted metadata.
- No dependency without license and advisory review.

### Verification

Phase 1:

- Unit tests for RFC 4648 vectors.
- Integration tests for round trips.
- `cargo clippy` with warnings denied.
- `cargo audit`.
- `cargo deny check`.
- `cargo license --json`.

Phase 2:

- Miri for scalar/in-place APIs.
- cargo-fuzz for malformed inputs, round trips, and differential tests.
- Kani proofs for bounded in-place decoding.

Phase 3:

- SIMD equivalence fuzzing.
- Per-architecture benchmark evidence.
- Release evidence archived under `target/release-evidence`.

## Ultimate Zero-Dependency Backlog

The following backlog captures the remaining features that can make
`base64-ng` a high-assurance, general-purpose replacement without weakening
the zero-runtime-dependency stance.

### Already Established

- Standard and URL-safe alphabets.
- Padded and unpadded engines.
- Strict-by-default canonical decoding.
- Explicit legacy/forgiving decode mode for whitespace-tolerant inputs.
- Caller-owned slice APIs.
- Allocation convenience APIs behind `alloc`.
- In-place encode and decode APIs.
- `std::io` streaming adapters.
- Constant-time-oriented validation and decode APIs for sensitive caller-owned
  buffers.
- Clear-tail APIs and streaming buffer cleanup for best-effort data retention
  reduction.
- Detailed decode errors with offsets.
- `no_std` scalar core.
- Const encode support into caller-sized arrays.
- Runtime backend reports and high-assurance scalar-only policy checks.
- Reserved SIMD backends that must prove scalar equivalence before admission.
- Named MIME, PEM, bcrypt-style, and `crypt(3)`-style profiles.
- Custom alphabet validation helpers with duplicate-character, padding-byte,
  and visible-ASCII checks.
- Stack-backed encoded and decoded output helpers for short values without
  `alloc`, including explicit visible-length, capacity, tail-clearing, and
  no-alloc ownership escape hatches.
- Internal best-effort wipe helpers for initialized bytes, vector spare
  capacity, and redacted `SecretBuffer` owned outputs when `alloc` is enabled.
- Zero-dependency `TryFrom<&str>`, `TryFrom<&[u8]>`, and byte-array interop
  for strict standard padded `EncodedBuffer` encoding plus `DecodedBuffer` and
  `SecretBuffer` decoding.
- Redacted-buffer comparison interop for bytes, byte-string literals, borrowed
  strings, and owned strings, routed through the same
  constant-time-oriented equal-length comparison helpers.
- Explicit `SecretBuffer` ownership escape hatches and `alloc`-gated
  conversions from stack-backed buffers into redacted owned storage.
- Strict line-wrapped in-place decode helpers for MIME/PEM-style profiles,
  including clear-tail variants and profile-level forwarding.
- README trust dashboard and CWE/security-control mapping documentation.
- Panic policy documentation and release-gated panic-like-site validation for
  non-test source.
- Checked quad reads and typed `[u8; 4]` scalar chunk helpers for strict,
  legacy, wrapped, and in-place decode paths.

### Remaining Long-Term Secure Core Work

- Continue replacing unchecked indexing where practical or documenting bounded
  internal indexing with proof, tests, or local invariants.

### Missing Performance Features

- Admit real AVX2, AVX-512, NEON, SSSE3/SSE4.1, and wasm `simd128` paths only
  after scalar differential tests, fuzz evidence, target-feature checks, unsafe
  inventory updates, and benchmark evidence are complete.
- Keep alignment and prefetch work internal and evidence-driven. Public
  alignment APIs are not admitted unless benchmarks show practical value and
  the API cannot cause unsafe caller assumptions.
- Keep scalar as the correctness reference and mandatory fallback for every
  accelerated backend.
- Maintain dependency-free benchmark harnesses first; only admit external
  benchmark tooling if the added dependency graph is justified.

### Admission-Gated Ecosystem Features

- Async/Tokio wrappers remain gated by `docs/ASYNC.md` and the dependency
  admission policy.
- `serde` integration remains optional and rejected by default until a concrete
  wrapper API and security story are reviewed.
- `bytes` integration remains optional and rejected by default until the
  dependency and trait surface are justified.
- Property-test crates remain outside the runtime dependency graph. The current
  preferred path is deterministic exhaustive tests and fuzz harnesses; add
  property-testing only in an isolated dev/harness context if it earns review.

## Roadmap

### v0.1

- Scalar strict encoding and decoding.
- Standard and URL-safe alphabets.
- Padded and unpadded modes.
- Caller-owned output buffers.
- Stable compile-time encoding into caller-sized arrays.
- `alloc` `Vec<u8>` and encoded `String` helpers.
- `std::io` streaming wrappers.
- Miri integrated into CI and the local release gate when installed.
- In-place encoding.
- In-place decoding.
- Hardened test/release scripts.

### v0.2

- Document panic-free public API expectations for scalar encode/decode paths.
- Add focused panic-free tests for malformed and size-boundary inputs.
- Strengthen bounded-memory documentation around checked length helpers and
  caller-owned output APIs.
- Explicit legacy decode mode.
- Migration guide for projects moving from the `base64` crate, including strict
  defaults and compatibility differences.
- More exhaustive malformed-input tests.
- Design an explicit constant-time scalar decode API for sensitive payloads,
  separate from the default fast strict decoder.

### v0.3

- Maintain isolated fuzz targets for arbitrary decode input, in-place decoding,
  stream chunk boundaries, and differential checks against established Base64
  implementations for canonical inputs.
- Provide dependency-free clear-tail encode/decode APIs for callers that want
  unused caller-owned output bytes cleared on success and full caller-owned
  output buffers cleared on error.
- Reconsider an optional `zeroize` feature only if users require a stronger
  best-effort cleanup primitive with a justified dependency tradeoff.
- Add release evidence documentation for audit, license, SBOM, and
  reproducibility artifacts.
- Document the unsafe SIMD admission bar before adding architecture-specific
  code, keeping the `simd` feature reserved until that evidence exists.
- Isolated scalar comparison benchmark harness first; consider Criterion only
  if its larger dependency graph is justified by better measurement quality.
- Prototype and verify the constant-time decode path with no secret-indexed
  table lookups during Base64 symbol mapping. Generated-code review remains
  required before making a formal cryptographic constant-time guarantee.

### v0.4

- AVX2 and NEON prototypes.
- Runtime feature dispatch.
- Scalar/SIMD differential testing.
- Cross-architecture CI evidence.
- Initial Kani scalar proof harnesses for length helpers and bounded in-place
  decode behavior.
- Internal scalar backend boundary as the reference path for future dispatch.
- Backend differential tests that compare dispatch behavior against the scalar
  reference for canonical, malformed, and undersized-buffer cases.
- Unsafe admission boundary in code and checks: crate-level `deny(unsafe_code)`
  with `allow(unsafe_code)` confined to the volatile wipe helpers and
  `src/simd.rs`.
- SIMD dispatch scaffold that detects AVX2/NEON candidates while keeping
  scalar as the only active backend.
- Inactive AVX2 fixed-block encode prototype with scalar-equivalence tests.
- Inactive NEON fixed-block encode prototype with scalar-equivalence tests for
  NEON-capable ARM targets.
- Public runtime backend report for audit logging and deployment assertions.
- Public runtime backend policy assertions for scalar-only and no-SIMD
  deployment requirements.
- High-assurance scalar-only backend policy for sensitive deployment profiles.
- Stable runtime enum string identifiers for audit logs and CI artifacts.
- Stable key/value runtime report and policy-failure display output.
- Constant-time-oriented clear-tail decode APIs for sensitive caller-owned
  buffers.
- Streaming encoder pending-buffer cleanup on consumption and drop.
- Streaming decoder pending-buffer and decoded-output queue cleanup on
  consumption and drop.
- Keep SIMD unsafe code isolated from the scalar core with documented invariants
  for every unsafe block.
- Maintain `docs/UNSAFE.md` as a central unsafe inventory for every admitted or
  prototype unsafe site.

### v0.5

- AVX-512 implementation.
- AVX-512 VBMI candidate detection for audit logs and future dispatch admission.
- CPU dispatch hardening.
- Keep the reserved `tokio`, `kani`, and `fuzzing` features inert and
  dependency-free until each feature has an admission review.
- Document the async-wrapper admission bar before adding a Tokio dependency.
- Streaming fuzz and regression tests for adjacent framed payloads.
- Release-gate hardening for packaged evidence, reserved feature placeholders,
  unsafe/SIMD boundary validation, and cross-target SIMD feature bundles.

### v0.6

- Completed profile-level support for MIME, PEM, and bcrypt-compatible
  alphabets where those profiles can remain strict, explicit, and
  dependency-free.
- Completed custom alphabet/profile construction with validation for duplicate
  symbols, padding conflicts, ASCII constraints, and deterministic errors.
- Completed line-wrapping encode support for PEM/MIME/common caller-selected
  wrapping policies, including CRLF and LF output.
- Completed validate-only APIs for strict, legacy, profile-aware, and
  constant-time-oriented validation use cases.
- Completed zero-dependency stack-backed output helpers for short encoded
  values.
- Completed internal best-effort wiping helpers and redacted `SecretBuffer`
  support for sensitive owned buffers.
- Completed the README trust dashboard and CWE/security-control mapping
  documentation.
- Strengthened Miri coverage with a shared check script for no-default scalar
  APIs and all-features alloc/stream APIs when nightly Miri is installed.
- Stabilized fuzz corpus handling with documented admission rules and a local
  corpus policy check.
- Completed the constant-time verification plan with generated-code review
  requirements and a release-gated non-claim wording check.
- Expanded gated Kani proof harness definitions for length helpers, in-place
  bounds, slice encode/decode bounds, and clear-tail decode cleanup. Execution
  remains gated until Kani supports the pinned Rust toolchain.
- Hardened scalar chunk validation and decode internals with checked quad reads
  and typed chunk helper inputs across strict, legacy, wrapped, and in-place
  decode paths.
- Hardened best-effort cleanup with audited volatile byte writes.
- Reduced constant-time-oriented padded terminal handling by removing explicit
  padding-count branch ladders from padded ct validation/decode paths.

### v0.7

- Scope this milestone as a scalar-only security-evidence release. Do not admit
  active SIMD dispatch in `v0.7`; keep hardware acceleration behind reporting,
  prototype, and evidence gates.
- Add a release-gated SIMD admission validator that keeps active dispatch
  scalar-only until scalar differential tests, fuzz evidence, unsafe inventory
  updates, benchmark evidence, and release-note wording are updated together.
- Add wasm `simd128` candidate reporting and reserved feature-bundle compile
  checks while keeping scalar as the only active backend.
- Add SSSE3/SSE4.1 candidate reporting and reserved feature-bundle compile
  checks for older x86 CPUs before active SIMD admission.
- Add an inactive SSSE3/SSE4.1 fixed-block encode prototype with scalar
  equivalence tests before any runtime admission.
- Add an isolated dudect-style timing harness for the scalar
  constant-time-oriented decoder. Keep timing runs opt-in, but compile and
  dependency-check the harness in CI and release gates.
- Generate release assembly artifacts for the scalar constant-time-oriented
  decoder so generated-code review has repeatable inputs.
- Add bounded Kani proof harnesses for constant-time-oriented decode result
  bounds, error cleanup, and validate/decode agreement.
- Evaluate alignment and prefetch optimizations only as internal
  benchmark-backed experiments, not as public contracts.
- Publish no SIMD acceleration claims for `v0.7`; release notes must describe
  candidate detection, inactive prototypes, and evidence gates only.

### v0.8

- Keep `v0.8` scalar-only unless active SIMD admission can include complete
  scalar differential tests, fuzz evidence, target-feature checks, unsafe
  inventory updates, benchmark evidence, and release-note wording in the same
  commit series.
- Keep inactive SIMD encode prototypes reserved. Replace them with real AVX2,
  AVX-512, NEON, SSSE3/SSE4.1, and wasm `simd128` candidate implementations
  only when their evidence is complete.
- Require every admitted SIMD implementation to document its vector-register
  retention cleanup strategy before it can become an active backend.
- Keep scalar as the default fallback and require runtime backend policy tests
  for every admitted accelerated backend.
- Publish per-architecture benchmark evidence for any performance claim,
  including CPU, OS, Rust version, command, and raw output.
- Consider a dependency-free helper macro or generator for audited custom
  alphabet encoders when an alphabet can be mapped without secret-indexed table
  access. Keep the fixed-scan fallback as the conservative default.
- Continue small native Rust interop only where it preserves explicit security
  semantics. Strict standard `SecretBuffer` `TryFrom`, stack-backed buffer
  conversions, explicit ownership escape hatches, and direct redacted-buffer
  comparisons are established; leave non-standard profiles on explicit
  engine/profile APIs.

### v0.9

- Consider admitting async streaming wrappers only after `docs/ASYNC.md`
  requirements are met, including dependency review, cancellation behavior,
  drop cleanup behavior, chunk-boundary tests, and release evidence with the
  async feature enabled.
- Consider optional `serde` and `bytes` integration only if a concrete user
  need clears dependency admission; otherwise keep both out of the crate.
- Continue native Rust interoperability that needs no dependencies, but avoid
  broad conversion traits when they would hide alphabet, padding, profile, or
  secret-handling choices.
- If async remains unjustified, keep `tokio` inert and spend this milestone on
  stream ergonomics, documentation, framed-protocol tests, and wasm/no-allocator
  portability checks.

### v0.10

- Release-candidate hardening for `v1.0`: API audit, documentation audit,
  migration guide refresh, fuzz corpus review, benchmark review, and release
  evidence rehearsal.
- Decide whether constant-time decode is formally guaranteed with supporting
  evidence or explicitly documented as constant-time-oriented only.
- Complete the panic-free public API audit for non-test scalar code and document
  any remaining bounded internal indexing with proof or test evidence.
- Freeze profile behavior for strict, legacy, MIME, PEM, bcrypt, custom
  alphabets, and validation-only APIs.
- Finalize the trust dashboard, CWE mapping, dependency admission outcomes, and
  security policy language for enterprise review.
- Freeze dependency policy and feature admission rules for `v1.0`.

### v1.0

- Kani proofs complete for scalar in-place decode.
- Formal or tool-backed evidence for panic-free scalar public APIs.
- Stable profile API for RFC 4648 standard and URL-safe, MIME, PEM, bcrypt, and
  custom alphabets.
- Stable validate-only APIs.
- Stable secret-buffer and best-effort cleanup API contracts.
- Published security and migration documentation for strict-by-default adoption.
- Constant-time decode guarantee either formally documented with supporting
  verification evidence or explicitly excluded from the stable API contract.
- Fuzz corpus stabilized.
- API freeze.
- Release gate mandatory.

## Release Gate

The release gate must pass before every release:

```sh
scripts/stable_release_gate.sh
```

Minimum required evidence:

- Formatting clean.
- Clippy clean.
- Tests pass under default, all-features, and no-default-features.
- Docs build.
- Dependency licenses accepted.
- RustSec advisories clean.
- Cargo license inventory generated.
- SBOM generated.
- Reproducible package/build check passes.

## Commit Policy

- Commit every completed, verified unit of work.
- Leave pushing to maintainers.
- Never hide failed checks. If a tool is missing locally, record that clearly.
