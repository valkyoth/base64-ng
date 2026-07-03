# SIMD Admission Manifest

This manifest is the release-facing checkpoint for hardware acceleration.
`base64-ng` may report SIMD candidates. Active accelerated dispatch is allowed
only for backends named in this file and the release gate.

## Current Admission State

- Admitted backends: AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode,
  NEON encode, AVX-512 VBMI strict decode, AVX2 strict decode,
  SSSE3/SSE4.1 strict decode, and NEON strict decode for std `x86`/`x86_64`
  or little-endian std `aarch64` where applicable.
- Active backend priority: AVX-512 VBMI, then AVX2, then SSSE3/SSE4.1 on
  x86/x86_64; NEON on little-endian aarch64; scalar otherwise.
- Runtime activation scope: std x86/x86_64 and little-endian std aarch64 dispatch only.
- Gate summary: Admitted backends: AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode, NEON encode, AVX-512 VBMI strict decode, AVX2 strict decode, SSSE3/SSE4.1 strict decode, and NEON strict decode.
- Gate priority: Active backend priority: AVX-512 VBMI, then AVX2, then SSSE3/SSE4.1 on x86/x86_64; NEON on little-endian aarch64.
- Public performance claims: none without local benchmark evidence.
- Release status: `1.3.2`; `1.2.0` admitted conservative active encode
  dispatch, and `1.3.0` admitted normal strict decode dispatch for the first
  narrow decode scope. `1.3.2` does not expand SIMD admission. Active encode
  dispatch admits AVX-512 VBMI above AVX2
  above SSSE3/SSE4.1 on x86/x86_64 and NEON on little-endian aarch64 for
  Standard and URL-safe alphabet families. AVX-512 VBMI strict decode is
  admitted above AVX2 and SSSE3/SSE4.1 strict decode for std
  `x86`/`x86_64` Standard and URL-safe alphabet families when a full 64-byte
  encoded block is present; AVX2 covers full 32-byte encoded blocks,
  SSSE3/SSE4.1 covers full 16-byte encoded blocks, and little-endian std
  `aarch64` NEON covers full 16-byte encoded blocks. Custom alphabets,
  big-endian AArch64, in-place decode, wrapped decode, legacy decode, CT
  secret decode, `no_std`, and wasm `simd128` decode remain scalar or
  prototype-only. Wrapped encode may use admitted fixed-block encode for its
  unwrapped staging step; line-ending insertion remains scalar.

The post-`1.3.2` non-standard surface review is tracked in
[SIMD_NON_STANDARD_SURFACE_REVIEW.md](SIMD_NON_STANDARD_SURFACE_REVIEW.md).
That ledger is not an admission record; it pins the current scalar/fallback
posture and lists evidence required before any broader surface can be
advertised.

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

## Wasm Posture Decision

For the `1.3.3` line, wasm `simd128` remains compile/codegen evidence only.
It is not admitted for runtime dispatch, and there is no admitted wasm runtime
profile. Candidate reporting may expose `wasm-simd128`, but active encode and
decode backends remain scalar on wasm32.

This is a deliberate decision: wasm execution passes through runtime/JIT
engines outside the crate's control, and this release does not have runtime
evidence for timing behavior, register retention, or the wasm wipe-barrier
caveat. The wasm32 wipe policy remains fail-closed unless callers explicitly
enable `allow-wasm32-best-effort-wipe`.

The detailed runtime decision is tracked in
[WASM_SIMD128_RUNTIME_REVIEW.md](WASM_SIMD128_RUNTIME_REVIEW.md).

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
| NEON | admitted backend | `neon` | little-endian std aarch64 runtime-dispatched encode and strict decode for Standard and URL-safe alphabet families; encode uses fixed 12-byte input blocks, and decode uses fixed 16-byte encoded blocks only after whole-input scalar validation preserves public error shape; shorter inputs, tails, unsupported alphabets, big-endian AArch64, 32-bit ARM, in-place encode/decode, wrapped decode, legacy decode, CT secret decode, line-ending insertion, and `no_std` use scalar fallback |
| wasm `simd128` | real non-dispatchable prototype | `simd128` | real fixed-block encode prototype for Standard and URL-safe alphabets; test-binary compile evidence only; non-dispatchable |

## Encode Surface Review

The `1.3.0` encode surface review keeps the active encode admission unchanged:
std x86/x86_64 AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and little-endian std
aarch64 NEON fixed-block encode for Standard and URL-safe alphabet families
only. Bcrypt, `crypt(3)`, custom alphabets, in-place encode, `no_std`
activation, and wasm runtime dispatch remain scalar or prototype-only. Wrapped
encode may route its unwrapped Base64 staging step through the admitted encode
boundary, but line-ending insertion itself remains scalar.

## Release Rule

Advertise SIMD acceleration only with the admitted backend name and scope. Do
not claim wasm `simd128`, custom alphabet, in-place, or any broader decode
acceleration until this manifest names those backends or API surfaces and
links to the matching differential, fuzz, unsafe, benchmark, and release-note
evidence.
