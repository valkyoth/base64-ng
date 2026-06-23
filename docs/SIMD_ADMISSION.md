# SIMD Admission Manifest

This manifest is the release-facing checkpoint for hardware acceleration.
`base64-ng` may report SIMD candidates. Active accelerated dispatch is allowed
only for backends named in this file and the release gate.

## Current Admission State

- Admitted backends: AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode, and
  NEON encode for std `x86`/`x86_64` and std `aarch64`.
- Active backend priority: AVX-512 VBMI, then AVX2, then SSSE3/SSE4.1 on
  x86/x86_64; NEON on aarch64; scalar otherwise.
- Runtime activation scope: std x86/x86_64 and std aarch64 dispatch only.
- Gate summary: Admitted backends: AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode, and NEON encode.
- Gate priority: Active backend priority: AVX-512 VBMI, then AVX2, then SSSE3/SSE4.1 on x86/x86_64; NEON on aarch64.
- Public performance claims: none without local benchmark evidence.
- Release status: `1.2.2`; `1.2.0` admitted conservative active encode
  dispatch, and `1.2.2` is an ergonomics and sanitization hardening patch for
  that released family. Active encode dispatch admits AVX-512 VBMI above AVX2
  above SSSE3/SSE4.1 on x86/x86_64 and NEON on aarch64 for Standard and
  URL-safe alphabet families. Decode, custom alphabets, in-place encode,
  `no_std`, and wasm `simd128` remain scalar or prototype-only. Wrapped encode
  may use admitted fixed-block encode for its unwrapped staging step;
  line-ending insertion remains scalar.

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

State labels are intentionally strict:

- `candidate only` means the build may report that the CPU feature bundle is
  visible, but runtime dispatch must still execute scalar code.
- `real non-dispatchable prototype` means fixed-block vector code exists for
  tests or generated evidence, but it is not called by public encode/decode
  APIs and is not represented by `ActiveBackend`.
- `admitted backend` means the backend is allowed to participate in runtime
  dispatch for the scope described in its evidence cell.

| Backend | State | Required CPU features | Evidence |
| --- | --- | --- | --- |
| AVX-512 VBMI | admitted backend | `avx512f`, `avx512bw`, `avx512vl`, `avx512vbmi` | std x86/x86_64 runtime-dispatched encode for Standard and URL-safe alphabet families; fixed 48-byte blocks use vector code only when at least one full block is present; shorter inputs, tails, unsupported alphabets, in-place encode, line-ending insertion, `no_std`, and decode use scalar fallback |
| AVX2 | admitted backend | `avx2` | std x86/x86_64 runtime-dispatched encode for Standard and URL-safe alphabet families; fixed 24-byte blocks use vector code only when at least one full block is present; shorter inputs, tails, unsupported alphabets, in-place encode, line-ending insertion, `no_std`, and decode use scalar fallback |
| SSSE3/SSE4.1 | admitted backend | `ssse3`, `sse4.1` | std x86/x86_64 runtime-dispatched encode for Standard and URL-safe alphabet families; fixed 12-byte blocks use vector code only when at least one full block is present; shorter inputs, tails, unsupported alphabets, in-place encode, line-ending insertion, `no_std`, and decode use scalar fallback |
| NEON | admitted backend | `neon` | std aarch64 runtime-dispatched encode for Standard and URL-safe alphabet families; fixed 12-byte blocks use vector code only when at least one full block is present; shorter inputs, tails, unsupported alphabets, 32-bit ARM, in-place encode, line-ending insertion, `no_std`, and decode use scalar fallback |
| wasm `simd128` | real non-dispatchable prototype | `simd128` | real fixed-block encode prototype for Standard and URL-safe alphabets; test-binary compile evidence only; non-dispatchable |

## Release Rule

Advertise SIMD acceleration only with the admitted backend name and scope. Do
not claim wasm `simd128`, custom alphabet, in-place, or decode
acceleration until this manifest names those backends or API surfaces and links to the matching
differential, fuzz, unsafe, benchmark, and release-note evidence.
