# base64-ng 2.0.0

`base64-ng` 2.0 is the completed API and package-family release built through
55 numbered, reviewable checkpoints from the signed 1.3.9 baseline.

## Highlights

- Adds validated codec specifications, strict RFC 4648 presets, transactional
  one-shot operations, a policy-carrying ordinary `Base64String`, a focused
  ordinary prelude, heapless incremental states, in-place transforms, bounded
  buffers, formatting, append, and chunk iterators.
- Adds bounded fixed-work secret encode/decode, explicit secret exposure,
  cleanup owners, protected-memory provider capabilities, backend health
  checks, and checked-backend quarantine and scalar retry.
- Makes sanitization dynamic secret decode bounded and fallibly allocated by
  default, with explicit protocol-specific limits and a 1,024-byte compile-time
  ceiling for stack staging. Derived encoded-input ceilings are enforced before
  constant-time-oriented validation across fixed, staged, locked, and bounded
  protected helpers, bounding attacker-controlled validation work as well as
  output allocation. The fixed-size policy error is non-exhaustive, and legacy
  locked-vector compatibility mappings for the 1 MiB default are documented.
- Extends admitted acceleration and evidence for x86, AArch64 NEON, wasm
  `simd128`, and exact-profile Linux/SpacemiT X60 RVV 1.0 encode and strict
  decode. Other RISC-V profiles remain scalar, and SVE remains non-dispatchable
  until native hardware evidence satisfies its admission contract.
- Rebuilds synchronous and Tokio adapters over canonical 2.0 state machines
  with explicit progress, backpressure, cancellation, cleanup, and checked
  recovery semantics.
- Publishes synchronized Serde, bytes, subtle, derive, sanitization, Tokio,
  MIME, PEM, multibase, IMAP, password-record, and OpenPGP companions.
- Bounds ordinary Serde compatibility decode to 1 MiB by default, adds
  protocol-selected const-generic limits, rejects oversized encoded input
  before full validation, and caps stack-backed ordinary fields at 4096 bytes.
- Makes dudect-style Welch evaluation fail closed on non-finite states and
  unequal zero-variance means, exact-pins `subtle` 2.6.1, and binds the locked
  harness tests into the subtle companion's focused release gate.
- Makes password-record work limits cumulative across validation and transform
  passes, admits generation work before allocation, and release-gates
  executable caller-owned behavior without `alloc`.
- Hardens RFC 7468 parsing with pre-allocation limits on both labels,
  cumulative parse/decode work accounting, exact padded secret sizing, and
  distinct malformed-input, resource, and internal-fault error classes.
- Keeps the PEM fuzz oracle aligned with cumulative work accounting through
  separate payload and document limits, release-gated parse headroom, and a
  deterministic oversized-seed regression test shared with the live target.
- Adds locked RFC and registry sources, semantic and interoperability corpora,
  fuzzing, sanitizer, Miri, Kani, timing, assembly, QEMU, package, SBOM, and
  reproducibility gates.
- Adds the supported `@valkyoth/base64-ng-wasm-loader` npm package with scalar
  and SIMD artifacts, runtime dispatch, checksums, source-commit provenance,
  and browser/runtime smoke evidence.
- Hardens publication with atomic success-only campaign manifests,
  tool-created reproducibility directories, source provenance bound to exact
  `HEAD`, mandatory npm provenance, and one pinned authorized SSH release
  signer shared by the Rust and npm publishers.
- Requires the native SpacemiT X60 RVV admission bundle at candidate and release
  time, validates its exact campaign source and hardware profile, rejects
  symlinked artifacts, and includes every bundle file in signed exact or
  metadata-equivalent release evidence.

## Migration

2.0 introduces validated crate-root APIs such as `Base64`,
`STRICT_STANDARD_PADDED`, and `STRICT_URL_SAFE_UNPADDED`. The internal `v2`
module is not public. Forgiving web decode,
legacy whitespace, wrapping, and protocol transforms remain explicitly named
opt-in surfaces. See [`docs/MIGRATION.md`](../docs/MIGRATION.md) for examples
and the complete 1.x-to-2.0 migration guide.

## Security Boundary

The default assurance provider is finite, volatile, generation-scoped, and
in-process only. Base 2.0 ships no persistent teardown provider and makes no
restart- or crash-recovery claim. Constant-time claims remain limited to the
documented fixed-work secret boundary and its exact evidence profile.

Publication requires the final Commit 55 pentest acceptance, green required
CI, complete release evidence, and the signed `v2.0.0` tag.
