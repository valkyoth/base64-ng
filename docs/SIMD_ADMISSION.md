# SIMD Admission Manifest

This manifest is the release-facing checkpoint for hardware acceleration.
`base64-ng` may report SIMD candidates, but active accelerated dispatch remains
forbidden until this file and the release gate are updated in the same commit as
the admitted implementation.

## Current Admission State

- Admitted backends: none.
- Active backend: scalar only.
- Public performance claims: none.
- Release status: `1.1.0` remains scalar-only. Future releases may admit an
  accelerated backend only when this manifest is updated with a complete
  backend admission evidence package in the same release series.

## Required For Every Admitted Backend

Before a backend is admitted, complete
[SIMD_ACTIVATION_CHECKLIST.md](SIMD_ACTIVATION_CHECKLIST.md). The checklist is
the contributor-facing expansion of this release manifest.

Each admitted backend must have all of the following evidence before it can be
added to `ActiveBackend` or used by runtime dispatch:

- scalar differential tests for canonical inputs, malformed inputs, undersized
  output buffers, strict padding, non-canonical trailing bits, and legacy
  whitespace profiles where applicable
- fuzz differential evidence against the scalar implementation
- target-feature compile checks for the exact CPU feature bundle
- runtime fallback tests proving unsupported CPUs execute scalar code
- unsafe inventory updates for every unsafe function and unsafe block
- register-retention cleanup strategy for any vector registers that process
  caller data
- explicit register cleanup implementation and tests for every vector path
  that processes caller data
- Miri coverage for all scalar and dispatch-level code Miri can execute
- benchmark evidence that reports hardware, OS, Rust version, command, raw
  output, and comparison baseline
- release-note wording that distinguishes admitted acceleration from candidate
  detection and avoids unsupported throughput claims

## Backend Rows

| Backend | State | Required CPU features | Evidence |
| --- | --- | --- | --- |
| AVX-512 VBMI | candidate only | `avx512f`, `avx512bw`, `avx512vl`, `avx512vbmi` | real fixed-block encode prototype for all alphabets; non-dispatchable |
| AVX2 | candidate only | `avx2` | real fixed-block encode prototype for Standard and URL-safe alphabets; non-dispatchable |
| SSSE3/SSE4.1 | candidate only | `ssse3`, `sse4.1` | real fixed-block encode prototype for Standard and URL-safe alphabets; non-dispatchable |
| NEON | candidate only | `neon` | real AArch64 fixed-block encode prototype for Standard and URL-safe alphabets; 32-bit ARM scaffold; non-dispatchable |
| wasm `simd128` | candidate only | `simd128` | real fixed-block encode prototype for Standard and URL-safe alphabets; test-binary compile evidence only; non-dispatchable |

## Release Rule

Do not advertise SIMD acceleration until this manifest names an admitted
backend and links to the matching differential, fuzz, unsafe, benchmark, and
release-note evidence.
