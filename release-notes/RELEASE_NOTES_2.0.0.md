# base64-ng 2.0.0

`base64-ng` 2.0 is the completed API and package-family release built through
55 numbered, reviewable checkpoints from the signed 1.3.9 baseline.

## Highlights

- Adds validated codec specifications, strict RFC 4648 presets, transactional
  one-shot operations, heapless incremental states, in-place transforms,
  bounded buffers, formatting, append, and chunk iterators.
- Adds bounded fixed-work secret encode/decode, explicit secret exposure,
  cleanup owners, protected-memory provider capabilities, backend health
  checks, and checked-backend quarantine and scalar retry.
- Extends admitted acceleration and evidence for x86, AArch64 NEON, and wasm
  `simd128`, while keeping RVV and SVE candidates non-dispatchable until native
  hardware evidence satisfies their admission contracts.
- Rebuilds synchronous and Tokio adapters over canonical 2.0 state machines
  with explicit progress, backpressure, cancellation, cleanup, and checked
  recovery semantics.
- Publishes synchronized Serde, bytes, subtle, derive, sanitization, Tokio,
  MIME, PEM, multibase, IMAP, password-record, and OpenPGP companions.
- Adds locked RFC and registry sources, semantic and interoperability corpora,
  fuzzing, sanitizer, Miri, Kani, timing, assembly, QEMU, package, SBOM, and
  reproducibility gates.
- Adds a supported npm wasm loader with scalar and SIMD artifacts, runtime
  dispatch, checksums, source-commit provenance, and browser/runtime smoke
  evidence.

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
