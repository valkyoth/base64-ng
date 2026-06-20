# base64-ng Professional Secure Plan

Date: 2026-05-14

## Objective

Build `base64-ng` as a modern, secure, `no_std`-first Base64 implementation for Rust. The crate must improve on the established `base64` crate only where evidence supports the claim: stricter release process, stronger verification, zero-copy ergonomics, optional streaming, and hardware acceleration that remains subordinate to proven scalar behavior.

## Current Baseline

- Rust stable/MSRV: `1.90.0`.
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
- The `allow-wasm32-best-effort-wipe` feature is allowed as a dependency-free
  policy switch for deployments that explicitly accept compiler-fence-only
  cleanup on `wasm32`; without it, `wasm32` builds fail closed.
- The `allow-compiler-fence-only-wipe` feature is allowed as a dependency-free
  policy switch for unsupported native architectures that explicitly accept
  compiler-fence-only cleanup after platform review; without it, those builds
  fail closed.

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
   - Future SSSE3/SSE4.1, AVX2, AVX-512 VBMI, ARM NEON, and wasm `simd128`.
   - Runtime dispatch only under `std` for the first admitted accelerated
     backends, because `std::is_x86_feature_detected!` provides the required
     CPU-feature gate on x86/x86_64.
   - `no_std` builds remain scalar-only unless a future API adds an explicit
     unsafe caller-side CPU contract. Compile-time target-feature reporting is
     not enough to dispatch safely on unknown hardware.
   - Unsafe isolated in `src/simd/` and documented.

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
- The scalar-side unsafe admissions remain small and release-gate enforced:
  volatile wipe helpers and CT comparison/result-gate barriers.
- Unsafe SIMD must live under dedicated modules.
- `allow(unsafe_code)` must remain confined to reviewed cleanup, CT, and SIMD
  helper files.
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

### Release Sequencing Assessment

The crate is feature-rich enough for normal Base64 use, and the remaining
`v1.0` question is assurance quality rather than feature volume. The `v0.10`,
`v0.11`, and `v0.12` lines completed the intended audit preparation,
verification hardening, and stabilization rehearsal work.

The current release path is:

- `v1.0.0`: stable API and security-contract release after the freeze
  candidate had clean local release evidence, clean CI, and clean external
  pentest results. This line should change only for release blockers,
  documentation corrections, or evidence-policy fixes before tagging.
- `1.0.x`: maintenance, assurance, and move-only source-layout releases. The
  current bounded no-default-features Kani harness set now runs on the pinned
  Rust `1.90.0` toolchain with `cargo-kani 0.67.0`; future `1.0.x` work may
  expand the harness scope, but the crate must still avoid claiming
  whole-crate or cryptographic formal verification.

The initial `1.0.0` contract accepted the documented Kani verifier exception in
[KANI.md](KANI.md). Current `1.0.x` evidence now includes a clean bounded Kani
harness run, but the crate must not claim Kani-complete or formally verified
cryptographic behavior until that stronger evidence exists.

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
  `src/simd/`.
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
  bounds, slice encode/decode bounds, and clear-tail decode cleanup. Current
  `1.0.x` evidence now runs these bounded harnesses on Rust `1.90.0` with
  `cargo-kani 0.67.0`.
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

- Treat this as a release-candidate audit preparation milestone, not the final
  pre-`v1.0` release by default.
- Perform a full public API audit: engines, profiles, stack buffers,
  `SecretBuffer`, stream adapters, runtime backend reporting, constant-time
  module, feature flags, and error variants.
- Classify every public API as stable-for-`v1.0`, experimental-but-retained,
  or removed/deferred before `v1.0`. Avoid new broad conversion traits or
  convenience APIs that hide alphabet, padding, profile, allocation, or secret
  handling choices.
- Refresh the documentation set as if a conservative security team will review
  it: README, migration guide, trust dashboard, CWE/security-control mapping,
  dependency policy, async policy, constant-time policy, SIMD admission policy,
  panic policy, unsafe inventory, release evidence, and benchmarks.
- Decide the `v1.0` constant-time wording. The default expectation is to keep
  the API explicitly "constant-time-oriented" unless the release has tool-backed
  generated-code and timing evidence strong enough for a formal guarantee.
- Keep default engine docs explicit that `STANDARD`, `URL_SAFE_NO_PAD`,
  wrapped profiles, bcrypt-style profiles, and custom strict decoders are not
  token-comparison or key-material decode APIs. Sensitive decode entry points
  are the `ct` constants and `Engine::ct_decoder()`.
- Freeze profile behavior for strict, legacy, MIME, PEM, bcrypt-style,
  `crypt(3)`-style, custom alphabets, wrapped profiles, and validation-only
  APIs.
- Freeze dependency policy and feature admission rules for the `v1.0` candidate
  series. `tokio`, `serde`, `bytes`, `zeroize`, `subtle`, property-test, and
  Criterion-style integrations remain deferred unless a written admission
  record justifies them.
- Rehearse release evidence with the stable release gate and record any skips
  or policy exceptions that would block `v1.0`.
- Defer a secure-decode marker trait or wrapper type to a post-`v1.0` design
  issue unless the final release-candidate audit proves the current `ct`
  constants and docs are insufficient.

### v0.11

- Verification and panic-policy hardening.
- Resolve Kani execution for the pinned Rust toolchain, pin a compatible Kani
  workflow, or document through [KANI.md](KANI.md) why a verifier exception is
  required and what evidence replaces it for `v1.0`.
  Completed direction: keep the Kani harnesses in-tree, document the initial
  `1.0.0` verifier exception, then enable the current bounded harness set once
  Rust `1.90.0` and `cargo-kani 0.67.0` were compatible.
- Expand or finalize proof harnesses for length helpers, slice encode/decode
  bounds, in-place decode bounds, clear-tail cleanup behavior, and
  constant-time-oriented validate/decode agreement.
- Add focused formal-verification coverage for the `decode_chunk` bit-packing
  logic so overflow and output-bound behavior are checked across all possible
  decoded 6-bit input combinations.
- Complete the panic-free public API audit for non-test scalar code. Document
  every remaining bounded internal index with proof, tests, or a local
  invariant in code or policy docs.
- Run focused fuzz campaigns for strict decode, legacy decode, in-place decode,
  wrapped profiles, custom alphabets, and stream chunk boundaries; keep corpus
  policy stable and dependency-isolated.
- Re-run generated-code review for constant-time-oriented paths and refresh
  assembly evidence. Keep dudect timing runs opt-in but make the evidence
  expectations explicit for release reviewers.
- Reassess best-effort cleanup claims against the volatile wipe implementation,
  stream queues, stack buffers, `EncodedBuffer`, `DecodedBuffer`, and
  `SecretBuffer`.

### v0.12

- Stabilization rehearsal for `v1.0`.
- Ship no broad new APIs unless the `v0.10` or `v0.11` audits found a
  correctness or security reason. Prefer removals, renames, documentation
  tightening, and test/evidence improvements over feature expansion.
- Run the migration guide against realistic examples from strict standard,
  URL-safe no-pad, MIME/PEM, legacy whitespace, custom alphabet, stack-buffer,
  secret-buffer, and stream use cases.
- Freeze MSRV policy and confirm `rust-toolchain.toml`, CI, docs.rs metadata,
  cross-target checks, Miri, Kani policy, fuzz harnesses, SBOM generation, and
  reproducibility checks agree.
- Do a final dependency admission review. Any optional ecosystem integration
  still without a concrete security and maintenance case remains out of
  `v1.0`.
- Publish release notes that explicitly describe the crate as a `v1.0`
  candidate and invite downstream API/security feedback before the final
  stable release.

### v1.0

- No unresolved `v0.10`, `v0.11`, or `v0.12` release-candidate blockers.
- `1.0.0` pentest, GitHub CI, and local release evidence must be clean
  before the stable `v1.0.0` tag.
- The initial stable release allowed the documented `v1.0` Kani verifier
  exception. Current `1.0.x` evidence includes a clean bounded Kani harness
  run, but this release line must not claim Kani-complete or formally verified
  cryptographic behavior.
- Formal or tool-backed evidence for panic-free scalar public APIs, including
  documented bounded-index invariants where indexing remains.
- Stable profile API for RFC 4648 standard and URL-safe, MIME, PEM, bcrypt, and
  custom alphabets.
- Stable validate-only APIs.
- Stable secret-buffer and best-effort cleanup API contracts.
- Published security and migration documentation for strict-by-default adoption.
- Constant-time decode guarantee either formally documented with supporting
  verification evidence or explicitly excluded from the stable API contract.
- Fuzz corpus stabilized.
- API freeze and feature-admission freeze.
- Release gate mandatory.

### Post-1.0 Direction

Post-`1.0` work should prefer assurance, measured evidence, and narrow API
admission over speed. Hardware acceleration is valuable only when the scalar
implementation remains the correctness reference and every unsafe backend has
clear admission evidence.

Before `1.1`, run a `1.0.x` source-layout series. These releases are intended
to help review and community contribution without changing behavior.

Rules for every source-layout release:

- Move code only; do not mix logic edits with file extraction.
- Preserve all public paths, re-exports, docs examples, and feature behavior.
- Review with moved-code-aware diffs.
- Run the full release gate, package verification, and public API audit.
- Treat any behavior change, new API, or security semantics change as out of
  scope for that split release.

Recommended `1.0.x` source-layout sequence:

- `1.0.2`: split `std::io` streaming adapters and stream tests into
  `src/stream/`, preserving `base64_ng::stream::*`.
- `1.0.3`: split runtime backend reporting and backend-policy types into
  `src/runtime/`; split alphabets, custom alphabet validation, the alphabet
  macro, and alphabet errors into `src/alphabet.rs`; split profile wrappers
  into `src/profiles.rs`; split best-effort cleanup helpers into
  `src/cleanup.rs`.
- `1.0.4`: split stack/owned buffer wrappers into `src/buffers/`.
- `1.0.5`: combined final source-layout cleanup before the pause window. Split
  constant-time-oriented decode, validation, masks, and CT barriers into
  `src/ct/`; split length helpers into `src/length.rs`; split wrapping and
  legacy transport handling into `src/wrap.rs`; split scalar core
  encode/decode helpers into `src/scalar.rs`; and split public error types into
  `src/errors.rs`.
- `1.0.6`: secure ergonomics release without adding dependencies. Add
  alloc-gated top-level strict standard `encode`/`decode` wrappers,
  `ct::CtEngine::decode_vec`, `ct::CtEngine::decode_secret`, public
  `constant_time_eq`, cookbook examples, and clearer idiomatic-conversion
  alphabet notes. Keep `serde` deferred as a future optional integration
  candidate until a concrete use case passes dependency admission review.
- `1.0.7`: assurance evidence release. Enable the current bounded
  no-default-features Kani harness set on Rust `1.90.0` with
  `cargo-kani 0.67.0`, strengthen constant-time-oriented byte accumulation,
  add AArch64 CSDB attestation posture reporting, expose memory-locking
  posture, and document streaming decoder partial-output semantics.
- `1.0.8`: audit-log and fail-closed maintenance release. Add redacted
  `DecodeErrorKind` classification, make stream decoder internal queue
  overflow paths latch failure like the encoder, split AArch64 CSDB
  build-asserted posture from native target guarantees, require runner-provided
  CI toolchains instead of unauthenticated rustup bootstrap, and refresh
  harness metadata.
- `1.0.9`: optional companion-crate release. Add
  `base64-ng-sanitization` as a separate workspace member for applications that
  already admit the `sanitization` crate and want CT decode helpers into
  clear-on-drop secret containers. Add `base64-ng-derive` as a dependency-free
  proc-macro companion for fixed-size `[u8; N]` tuple newtypes, and expose
  `clear_bytes` from the core crate so generated cleanup can reuse the reviewed
  wipe path without emitting unsafe code into downstream crates. Add
  `base64-ng-serde`, `base64-ng-bytes`, and `base64-ng-tokio` as explicit
  opt-in ecosystem companion crates. Keep the core crate dependency-free.
- `1.1.x`: add `base64-ng-subtle` as an explicit companion crate for
  applications that already admit `subtle` and want reviewed
  `ConstantTimeEq` comparison helpers without adding `subtle` to the core
  crate.
- `1.0.10`: source-layout maintenance release. Split oversized production
  modules into focused internal files, move Kani proofs and unit tests out of
  the root module, and add a 500-line production-source budget guard to the
  normal check pipeline. Keep the public API, scalar-only runtime behavior, and
  companion crate versions unchanged.
- After `1.0.10`: park core feature work for a few weeks so users can test the
  stable API and report issues before any `1.1` SIMD-admission work starts.

The recommended post-`1.0` SIMD path is incremental. Minor versions should
mean something visible to users: `1.1.x` is the non-accelerated SIMD encode
evidence series, `1.2.0` is the first release that may activate encode
acceleration if the evidence is complete, `1.2.x` is the non-accelerated SIMD
decode evidence series, and `1.3.0` is the first release that may activate both
encode and decode acceleration if the decode evidence is complete. After each
active-acceleration minor release, pause feature work for a short soak period
so users can report platform-specific regressions before the next acceleration
line begins.

Current `1.1.x` checkpoint state:

- `1.1.0`: started the SIMD encode foundation line with real non-dispatchable
  SSSE3/SSE4.1 fixed-block encode logic for Standard and URL-safe alphabets.
- `1.1.1`: hardened the SIMD encode prototype evidence line with AVX-512,
  AVX2, NEON, wasm `simd128`, generated assembly evidence, backend evidence,
  and the future encode-admission draft while keeping runtime dispatch
  scalar-only.
- `1.1.2`: added the optional `base64-ng-subtle` companion crate and tightened
  SIMD prototype cleanup evidence, including inline AArch64 NEON register
  cleanup for the test-only prototype path.
- Next `1.1.x` checkpoints: refresh documentation and machine gates so
  release evidence consistently distinguishes real non-dispatchable prototypes
  from admitted active backends, then run pentest/CI/Kani on the candidate
  before tagging the next GitHub checkpoint. Do not publish additional
  crates.io versions for this line unless a user-impacting fix requires it.

Remaining before `1.2.0` active encode dispatch can be considered:

- Keep `active_backend()` returning scalar and keep `ActiveBackend` without
  accelerated variants until the admission manifest is updated in the same
  commit as the admitted dispatch code.
- Run the complete scalar differential, fuzz, Miri, Kani, dudect-style,
  generated assembly, backend evidence, unsafe-boundary, panic-policy,
  dependency, package, and release metadata gates on the exact release
  candidate.
- Produce per-backend hardware evidence for any backend considered for active
  dispatch. A backend may stay real-but-non-dispatchable in `1.2.0` if its
  hardware evidence is incomplete.
- Produce benchmark evidence with hardware, OS, Rust version, feature flags,
  command, raw output, scalar baseline, and exact admitted backend names.
- Update `docs/SIMD_ADMISSION.md`, runtime report expectations, release notes,
  unsafe inventory, benchmark docs, and the release script in the same change
  that admits any backend.
- Keep `no_std` acceleration disabled unless a future unsafe caller-contract
  API is designed, reviewed, and separately admitted.
- Publish the next crates.io family sync as `1.2.0` only after the `1.1.x`
  checkpoint tags and user testing remain clean.

After `1.2.0`:

- Pause encode feature work for roughly two weeks before starting SIMD decode
  work.
- Build SIMD decode prototypes as a separate non-accelerated `1.2.x` line.
  Start with strict Standard and URL-safe decode only. Keep prototypes
  non-dispatchable while proving invalid-byte handling, canonical trailing-bit
  rejection, padding behavior, output-retention cleanup, error-shape
  compatibility where promised, scalar differential tests, fuzz coverage, and
  timing-oriented evidence.
- Consider active SIMD decode dispatch in `1.3.0` only if the `1.2.x` decode
  evidence line is complete and the `1.2.0` encode acceleration line has had a
  clean soak period. If admitted, release notes must distinguish encode
  backends from decode backends and keep `HighAssuranceScalarOnly` available
  for deployments that prefer scalar execution.

SIMD admission rules for all post-`1.0` work:

- Start with encode, not decode.
- Start with Standard and URL-safe alphabets. Bcrypt, `crypt(3)`, and custom
  alphabets remain scalar unless an accelerated mapper is proven correct and
  does not introduce secret-indexed lookup behavior outside documented
  non-secret contexts.
- Keep prototypes non-dispatchable until they have scalar differential tests,
  fuzz coverage, generated assembly review, register-retention cleanup,
  target-feature checks, and benchmark evidence.
- Keep `no_std` acceleration disabled unless a future unsafe API makes the CPU
  contract explicit at the call site.
- Do not publish performance claims until the active backend, benchmark output,
  hardware details, Rust version, feature flags, and release evidence all match
  the claim.
- Preserve `runtime::BackendPolicy::HighAssuranceScalarOnly` as the recommended
  deployment policy for users who prefer the audited scalar path over any
  accelerated backend, while keeping platform-specific speculation-barrier
  attestation outside the crate when the runtime report marks a CT gate as
  unattested.

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
