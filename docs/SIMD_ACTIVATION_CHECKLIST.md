# SIMD Activation Checklist

This checklist is mandatory before any additional SIMD code becomes
dispatchable. AVX-512 VBMI, AVX2, SSSE3/SSE4.1, NEON, and the narrow wasm
`simd128` runtime profile are admitted only for their documented Standard and
URL-safe alphabet-family scopes. Custom-alphabet, in-place, wrapped, legacy,
CT-secret, broader wasm/browser, and future decode surfaces remain
prototype-only or scalar until their own checklist evidence is complete.

## Non-Negotiable Rule

Do not add another accelerated backend to `ActiveBackend`, runtime dispatch,
public performance claims, or release notes until every item below is complete
in the same release series.

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

For NEON specifically, "used registers" cannot be guessed from source-level
local variable count. Admission evidence must include optimized generated
assembly for the exact target and feature bundle. If the compiler allocates
caller-derived values to callee-saved vector registers, the admission package
must also account for any compiler-generated spill/restore slots and prove that
those slots are wiped or never contain caller-derived data.

For Windows x64 specifically, XMM6 through XMM15 are callee-saved. AVX2 or
AVX-512 admission on MSVC targets must prove that compiler-generated
callee-saved XMM spill/restore slots do not retain caller-derived vector
values, or use an ABI boundary that prevents those spills from carrying
secret-bearing intermediates.

For wasm `simd128`, source-level register cleanup is not sufficient evidence.
The wasm runtime/JIT owns final register allocation and may rewrite the
generated code. A wasm backend cannot be admitted until the selected runtime
and deployment profile have generated-code/JIT evidence, or the release notes
scope wasm SIMD out of any register-retention claim.

Current non-dispatchable prototypes are exempt from admission only because they
are test-only and non-dispatchable. The narrow wasm `simd128` profile is no
longer prototype-only: it is admitted only for the documented runtime smoke
profile and remains scoped out of hardware-like register-retention and
runtime/JIT timing claims.

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
