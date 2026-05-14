# base64-ng Professional Secure Plan

Date: 2026-05-13

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
- Fuzzing crates may live only in a future `fuzz/` workspace or tool-specific harness, not in normal runtime dependencies.
- Benchmark crates may live only in `dev-dependencies` or an isolated bench workspace.
- Kani support must stay feature-gated and must not affect normal users.

Rejected by default:

- Helper crates for simple bit manipulation, table generation, feature selection, error formatting, or CLI tooling.
- Git dependencies.
- Runtime dependencies for the default feature set.
- Dependencies with unclear licensing, advisories, yanked releases, or unnecessary transitive graphs.

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
   - Future async wrappers.
   - Explicit optional dependency.

### API Policy

- Stable Rust only for public APIs.
- No `generic_const_exprs` or unstable const-generic tricks in the public surface.
- Use zero-sized engines and trait-based alphabets.
- Strict decoding is default.
- Legacy behavior must be opt-in and named.

## Security Design

### Hard Rules

- Scalar code denies unsafe.
- Unsafe SIMD must live under dedicated modules.
- `allow(unsafe_code)` must remain confined to `src/simd.rs`.
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
  with `allow(unsafe_code)` confined to `src/simd.rs`.
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
- Keep SIMD unsafe code isolated from the scalar core with documented invariants
  for every unsafe block.
- Maintain `docs/UNSAFE.md` as a central unsafe inventory for every admitted or
  prototype unsafe site.

### v0.5

- AVX-512 implementation.
- CPU dispatch hardening.
- Tokio streaming wrappers.
- Async cancellation and partial-read tests.
- Streaming fuzz and regression tests for adjacent framed payloads.

### v1.0

- Kani proofs complete for scalar in-place decode.
- Formal or tool-backed evidence for panic-free scalar public APIs.
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
