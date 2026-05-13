# SIMD Admission Policy

`base64-ng` is intentionally scalar-only in the `0.3` line. On `main`, the
crate uses `#![deny(unsafe_code)]` and permits `allow(unsafe_code)` only in the
private `src/simd.rs` boundary. The `simd` feature remains reserved until
architecture-specific code has enough evidence to justify enabling it.

This is a security decision, not a rejection of hardware acceleration. SIMD
must be added only when it can be isolated, tested, and reviewed without
weakening the scalar trust base.

## Current Status

- No unsafe code is compiled by the published crate.
- `scripts/validate-unsafe-boundary.sh` verifies that `allow(unsafe_code)` is
  confined to `src/simd.rs`.
- The scalar implementation is the reference behavior.
- Encode and decode entry points already pass through an internal backend
  boundary, currently backed only by the scalar implementation.
- Unit tests compare dispatch behavior against the scalar reference for
  canonical inputs, malformed inputs, and undersized output buffers.
- The `simd` feature does not enable accelerated code yet.
- CI checks the reserved `simd` feature in `no_std` mode for x86_64, aarch64,
  wasm32, and Cortex-M targets.
- Performance claims must be backed by local benchmark evidence, not roadmap
  language.

Run the same target check locally for every installed target:

```sh
scripts/check_targets.sh
```

Run a specific target:

```sh
scripts/check_targets.sh aarch64-unknown-linux-gnu
```

## Required Before SIMD Code Lands

Any AVX2, NEON, AVX-512, or runtime-dispatch implementation must include:

- The dedicated `src/simd.rs` boundary for all architecture-specific code.
- Crate-level `deny(unsafe_code)` must continue to reject unsafe outside the
  SIMD module.
- A local safety comment for every unsafe block.
- Deterministic differential tests against scalar encode/decode behavior.
- Fuzz differential coverage for strict and legacy-compatible inputs where
  applicable.
- Runtime dispatch tests that prove unsupported CPUs fall back to scalar.
- Miri coverage for scalar and dispatch-level code that Miri can execute.
- Architecture-specific CI evidence or documented local evidence for each
  enabled target.
- Benchmark evidence that reports hardware, OS, Rust version, command, and raw
  output.

## Dispatch Rules

- Scalar remains the fallback for every build.
- Runtime CPU detection may be used only behind `std`.
- Compile-time target-feature paths must be explicit and documented.
- Unsupported CPU features must never panic at runtime.
- SIMD paths must preserve strict error indexes, canonical padding rejection,
  and output sizing behavior.

## Release Rule

Do not advertise SIMD acceleration in release notes until accelerated code is
actually enabled, tested, and measured for that release.
