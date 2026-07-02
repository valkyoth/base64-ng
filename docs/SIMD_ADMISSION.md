# SIMD Admission Manifest

This manifest is the release-facing checkpoint for hardware acceleration.
`base64-ng` may report SIMD candidates. Active accelerated dispatch is allowed
only for backends named in this file and the release gate.

## Current Admission State

- Admitted backends: AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode,
  NEON encode, AVX-512 VBMI strict decode, AVX2 strict decode, and
  SSSE3/SSE4.1 strict decode for std `x86`/`x86_64` or std `aarch64` where
  applicable.
- Active backend priority: AVX-512 VBMI, then AVX2, then SSSE3/SSE4.1 on
  x86/x86_64; NEON on aarch64; scalar otherwise.
- Runtime activation scope: std x86/x86_64 and std aarch64 dispatch only.
- Gate summary: Admitted backends: AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode, NEON encode, AVX-512 VBMI strict decode, AVX2 strict decode, and SSSE3/SSE4.1 strict decode.
- Gate priority: Active backend priority: AVX-512 VBMI, then AVX2, then SSSE3/SSE4.1 on x86/x86_64; NEON on aarch64.
- Public performance claims: none without local benchmark evidence.
- Release status: `1.2.3`; `1.2.0` admitted conservative active encode
  dispatch, and `1.2.3` is a dependency-sync patch for the optional
  sanitization companion. Active encode dispatch admits AVX-512 VBMI above
  AVX2 above SSSE3/SSE4.1 on x86/x86_64 and NEON on aarch64 for Standard and
  URL-safe alphabet families. In the `1.3.0` working line, AVX-512 VBMI strict
  decode is admitted above AVX2 and SSSE3/SSE4.1 strict decode for std
  `x86`/`x86_64` Standard and URL-safe alphabet families when a full 64-byte
  encoded block is present; AVX2 covers full 32-byte encoded blocks and
  SSSE3/SSE4.1 covers full 16-byte encoded blocks. NEON, custom alphabets,
  in-place decode, wrapped decode, legacy decode, CT secret decode, `no_std`,
  and wasm `simd128` decode remain scalar or prototype-only. Wrapped encode
  may use admitted fixed-block encode for its
  unwrapped staging step; line-ending insertion remains scalar.

## `1.3.0` Decode Admission Scope Freeze

The first decode acceleration line is intentionally narrower than the full
decode API. Any `1.3.0` decode backend admission must start with strict
Standard and URL-safe alphabets only, for padded and unpadded inputs, through
the normal strict decode backend boundary. The following surfaces remain
scalar unless a later evidence package separately admits them:

- line-wrapped MIME/PEM decode
- legacy whitespace-tolerant decode
- bcrypt-style and `crypt(3)`-style profiles
- custom alphabets
- `no_std` SIMD dispatch
- wasm `simd128` runtime dispatch
- in-place decode
- constant-time-oriented `base64_ng::ct` secret decode

This scope is frozen before implementation work starts so security review can
separate normal strict decode acceleration from the constant-time-oriented
scalar path. Future normal SIMD decode must not be routed into `ct::CtEngine`
or advertised as a secret-decoding acceleration path without a separate formal
side-channel evidence package.

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
| AVX-512 VBMI | admitted backend | `avx512f`, `avx512bw`, `avx512vl`, `avx512vbmi` | std x86/x86_64 runtime-dispatched encode and strict decode for Standard and URL-safe alphabet families; encode uses fixed 48-byte input blocks, and decode uses fixed 64-byte encoded blocks only after whole-input scalar validation preserves public error shape; shorter inputs fall back to AVX2, SSSE3/SSE4.1, or scalar, tails use scalar, and unsupported alphabets, in-place encode/decode, wrapped decode, legacy decode, CT secret decode, line-ending insertion, and `no_std` use scalar fallback |
| AVX2 | admitted backend | `avx2` | std x86/x86_64 runtime-dispatched encode and strict decode for Standard and URL-safe alphabet families; encode uses fixed 24-byte input blocks, and decode uses fixed 32-byte encoded blocks only after whole-input scalar validation preserves public error shape; shorter inputs fall back to SSSE3/SSE4.1 or scalar, tails use scalar, and unsupported alphabets, in-place encode/decode, wrapped decode, legacy decode, CT secret decode, line-ending insertion, and `no_std` use scalar fallback |
| SSSE3/SSE4.1 | admitted backend | `ssse3`, `sse4.1` | std x86/x86_64 runtime-dispatched encode and strict decode for Standard and URL-safe alphabet families; encode uses fixed 12-byte input blocks, and decode uses fixed 16-byte encoded blocks only after whole-input scalar validation preserves public error shape; shorter inputs, tails, unsupported alphabets, in-place encode/decode, wrapped decode, legacy decode, CT secret decode, line-ending insertion, and `no_std` use scalar fallback |
| NEON | admitted backend | `neon` | std aarch64 runtime-dispatched encode for Standard and URL-safe alphabet families; fixed 12-byte encode blocks use vector code only when at least one full block is present; a fixed 16-byte decode block prototype exists for tests and evidence only; public decode through NEON remains prototype-only, while shorter inputs, tails, unsupported alphabets, 32-bit ARM, in-place encode, line-ending insertion, and `no_std` use scalar fallback |
| wasm `simd128` | real non-dispatchable prototype | `simd128` | real fixed-block encode prototype for Standard and URL-safe alphabets; test-binary compile evidence only; non-dispatchable |

## Release Rule

Advertise SIMD acceleration only with the admitted backend name and scope. Do
not claim wasm `simd128`, custom alphabet, in-place, NEON decode, or any
broader decode acceleration until this manifest names those backends or API
surfaces and links to the matching differential, fuzz, unsafe, benchmark, and
release-note evidence.
