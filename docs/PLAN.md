# base64-ng Professional Secure Plan

Date: 2026-05-13

## Objective

Build `base64-ng` as a modern, secure, `no_std`-first Base64 implementation for Rust. The crate must improve on the established `base64` crate only where evidence supports the claim: stricter release process, stronger verification, zero-copy ergonomics, optional streaming, and hardware acceleration that remains subordinate to proven scalar behavior.

## Current Baseline

- Rust stable: `1.95.0`.
- License: `MIT OR Apache-2.0`.
- Project name: `base64-ng`.
- Runtime and dev dependency graph: zero external crates.
- Local testing system modeled after `fluxheim`: check script, release gate, dependency policy, audit config, SBOM, reproducible build check, and CI.

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
   - Future `Vec<u8>` and `String` helpers.
   - Optional feature.

3. `simd`
   - Future AVX2, AVX-512, and ARM NEON.
   - Runtime dispatch under `std`.
   - Compile-time target-feature paths for embedded or specialized builds.
   - Unsafe isolated and documented.

4. `stream`
   - Future `std::io::{Read, Write}` wrappers.
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

- Scalar code forbids unsafe.
- Unsafe SIMD must live under dedicated modules.
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
- In-place decoding.
- Hardened test/release scripts.

### v0.2

- `alloc` convenience APIs.
- Explicit legacy decode mode.
- More exhaustive malformed-input tests.
- Miri integration.

### v0.3

- AVX2 and NEON prototypes.
- Runtime feature dispatch.
- Criterion benchmarks.
- Scalar/SIMD differential testing.

### v0.4

- AVX-512 implementation.
- CPU dispatch hardening.
- Cross-architecture CI evidence.

### v0.5

- `std::io::Read` streaming decoder.
- Broader chunk-boundary state machine tests.

### v0.6

- Tokio streaming wrappers.
- Async cancellation and partial-read tests.

### v1.0

- Kani proofs complete for scalar in-place decode.
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
