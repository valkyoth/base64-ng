# SIMD Admission Policy

`base64-ng` is intentionally scalar-only in the `0.3` line. The crate currently
uses `#![forbid(unsafe_code)]`, and the `simd` feature is reserved until unsafe
architecture-specific code has enough evidence to justify changing that policy.

This is a security decision, not a rejection of hardware acceleration. SIMD
must be added only when it can be isolated, tested, and reviewed without
weakening the scalar trust base.

## Current Status

- No unsafe code is compiled by the published crate.
- The scalar implementation is the reference behavior.
- The `simd` feature does not enable accelerated code yet.
- CI checks the reserved `simd` feature in `no_std` mode for x86_64, aarch64,
  wasm32, and Cortex-M targets.
- Performance claims must be backed by local benchmark evidence, not roadmap
  language.

## Required Before SIMD Code Lands

Any AVX2, NEON, AVX-512, or runtime-dispatch implementation must include:

- A dedicated module boundary for all architecture-specific code.
- A deliberate change from crate-wide `forbid(unsafe_code)` to a policy that
  still denies unsafe outside the SIMD module.
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
