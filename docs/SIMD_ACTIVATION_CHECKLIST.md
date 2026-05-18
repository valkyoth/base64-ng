# SIMD Activation Checklist

This checklist is mandatory before any SIMD code becomes dispatchable. Current
SIMD prototypes are scaffolding only: they zero destination buffers with SIMD
and then overwrite those buffers with scalar encoding.

## Non-Negotiable Rule

Do not add an accelerated backend to `ActiveBackend`, runtime dispatch, public
performance claims, or release notes until every item below is complete in the
same release series.

## Register Cleanup Admission Gate

This is a hard blocker for every real SIMD backend. A vector path that loads,
shuffles, masks, compares, or table-lookups caller bytes must not become
dispatchable until it has an explicit register-retention cleanup strategy.

The admission evidence must include:

- A list of every SIMD register class and register number that may carry
  caller data in the function body.
- A cleanup sequence immediately before every return path from the SIMD
  function.
- Generated assembly showing the cleanup sequence is present in optimized
  builds for the exact target-feature bundle being admitted.
- A statement of what the cleanup does and does not claim. Register cleanup is
  data-retention reduction inside the current thread context; it is not a
  formal microarchitectural side-channel proof.
- A review update in `docs/UNSAFE.md` for the admitted backend.

Architecture-specific baseline requirements:

- AVX-512: clear every used secret-bearing ZMM/YMM/XMM register before return
  and include the appropriate AVX transition cleanup, such as `vzeroupper`,
  when returning to code that may use narrower vector state.
- AVX2: clear every used secret-bearing YMM/XMM register before return and
  include `vzeroupper` where applicable.
- SSSE3/SSE4.1: clear every used secret-bearing XMM register before return.
- NEON: clear every used secret-bearing V/Q register before return.
- wasm `simd128`: document the runtime's register-retention limitations and
  provide generated-code evidence for the selected wasm toolchain/runtime.

Current prototypes are exempt only because they construct zero vectors and do
not load caller bytes into SIMD registers. That exemption ends the moment a
prototype starts processing caller input with vector registers.

## Source Changes

- Keep scalar encode/decode as the reference implementation.
- Add only the intrinsics used by the admitted algorithm. Imported intrinsics
  are not evidence by themselves.
- Document every unsafe function and unsafe block in `docs/UNSAFE.md`.
- Implement and explain vector register cleanup for every SIMD path that
  handles caller data. Missing cleanup is a release blocker, not a TODO.
- Keep scalar fallback behavior for unsupported targets and feature sets.
- For `no_std`, do not dispatch from compile-time target-feature reporting
  alone unless the API includes an explicit caller-side CPU contract.

## Correctness Evidence

- Add scalar differential tests for canonical inputs.
- Add malformed-input differential tests.
- Add undersized-output differential tests.
- Add strict padding and non-canonical trailing-bit tests.
- Add profile coverage for wrapped, legacy whitespace, URL-safe, bcrypt-style,
  `crypt(3)`-style, and custom alphabets where applicable.
- Add fuzz differential evidence against the scalar implementation.
- Add deterministic edge-case vectors for block boundaries, tails, and empty
  inputs.

## Security Evidence

- Run Miri for scalar and dispatch-level code that Miri can execute.
- Run the unsafe-boundary validator and update its allowlist only with review.
- Run dudect/constant-time evidence for sensitive scalar fallbacks and any
  constant-time SIMD path that is claimed.
- Review generated assembly for dispatch, tail handling, register cleanup, and
  constant-time-sensitive code.
- Keep `candidate_detection_mode` accurate in runtime reports and release
  evidence.

## Platform Evidence

- Compile the exact target-feature bundle for each backend.
- For `std` x86/x86_64, test runtime CPU-feature fallback behavior.
- For `no_std`, document whether acceleration is disabled or requires an
  explicit unsafe caller contract.
- Run backend evidence capture on hardware that actually supports the backend.
- Record OS, CPU model, Rust version, target triple, `RUSTFLAGS`, and commands.

## Release Evidence

- Update `docs/SIMD_ADMISSION.md` backend rows from candidate-only to admitted.
- Update `docs/SIMD.md`, `docs/UNSAFE.md`, `docs/RELEASE_EVIDENCE.md`, and
  `docs/TRUST.md`.
- Update release notes with precise, measured claims only.
- Include benchmark output with scalar baseline and hardware details.
- Keep cargo-audit, cargo-deny, cargo-license, fuzz, Miri, and cross-target
  checks green.

## Final Review Questions

- Does the accelerated path produce exactly the same output and errors as the
  scalar reference for every supported profile?
- Can unsupported CPUs execute without illegal instructions?
- Is every data-dependent branch, lookup, and tail path intentional and
  documented?
- Are register-retention and temporary-buffer cleanup handled or explicitly
  scoped out?
- Would a release auditor understand what is proven, what is measured, and
  what remains a non-claim?
