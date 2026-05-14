# Unsafe Code Inventory

`base64-ng` keeps unsafe code out of the scalar implementation. The crate root
uses `#![deny(unsafe_code)]`, and the only source file allowed to lower that
lint is `src/simd.rs`.

This inventory is intentionally small and release-gate enforced. Any new unsafe
block must be added here before an accelerated backend can be admitted.

## Policy

- Default builds compile no unsafe code.
- Optional SIMD prototypes live only in `src/simd.rs`.
- `scripts/validate-unsafe-boundary.sh` fails if `allow(unsafe_code)` appears
  outside `src/simd.rs`.
- `scripts/validate-unsafe-boundary.sh` fails if architecture intrinsics, CPU
  feature detection, or `target_feature` gates appear outside `src/simd.rs`.
- Every unsafe function and unsafe block must have a local safety explanation.
- Prototype functions are not eligible for runtime dispatch.

## Current Unsafe Sites

### `encode_48_bytes_avx512`

Location: `src/simd.rs`

Status: inactive prototype, not dispatchable.

Purpose:

- Exercise AVX-512 target-feature plumbing.
- Validate the unsafe boundary before an admitted AVX-512 path exists.
- Provide scalar-equivalence test coverage before any real vector path is
  admitted.

Preconditions:

- Caller must prove the full AVX-512 Base64 candidate bundle is available on
  the current CPU: `avx512f`, `avx512bw`, `avx512vl`, and `avx512vbmi`.
- Input is exactly 48 bytes.
- Output is exactly 64 bytes.

Unsafe operation:

- `_mm512_storeu_si512` stores one 512-bit zero vector into the output buffer.

Safety argument:

- The output type is `&mut [u8; 64]`, so the store has enough initialized,
  writable memory.
- The intrinsic is the unaligned store variant, so no stronger alignment is
  required.
- The function is guarded by the full AVX-512 Base64 target-feature contract.
- The prototype then overwrites the block with scalar-equivalent Base64 output.

### `encode_24_bytes_avx2`

Location: `src/simd.rs`

Status: inactive prototype, not dispatchable.

Purpose:

- Exercise AVX2 target-feature plumbing.
- Validate the unsafe boundary.
- Provide scalar-equivalence test coverage before any real vector path is
  admitted.

Preconditions:

- Caller must prove AVX2 is available on the current CPU.
- Input is exactly 24 bytes.
- Output is exactly 32 bytes.

Unsafe operation:

- `_mm256_storeu_si256` stores one 256-bit zero vector into the output buffer.

Safety argument:

- The output type is `&mut [u8; 32]`, so the store has enough initialized,
  writable memory.
- The intrinsic is the unaligned store variant, so no stronger alignment is
  required.
- The function is guarded by an AVX2 target-feature contract.
- The prototype then overwrites the block with scalar-equivalent Base64 output.

### `encode_12_bytes_neon`

Location: `src/simd.rs`

Status: inactive prototype, not dispatchable.

Purpose:

- Exercise ARM NEON intrinsic plumbing.
- Validate the unsafe boundary on ARM targets.
- Provide scalar-equivalence test coverage before any real vector path is
  admitted.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- NEON is mandatory on `aarch64`; `arm` builds must enable the `neon` target
  feature.
- Input is exactly 12 bytes.
- Output is exactly 16 bytes.

Unsafe operations:

- `vdupq_n_u8` constructs one 128-bit NEON vector.
- `vst1q_u8` stores that vector into the output buffer.

Safety argument:

- The output type is `&mut [u8; 16]`, so the store has enough initialized,
  writable memory.
- The function is compiled only for `aarch64` or `arm` builds with the `neon`
  target feature.
- The function's safety contract requires runtime NEON availability.
- The prototype then overwrites the block with scalar-equivalent Base64 output.

## Admission Rule

Unsafe SIMD can become an active backend only after scalar differential tests,
fuzz evidence, architecture-specific build evidence, benchmark evidence, and
review of this inventory all pass for that release.

Any admitted SIMD path that processes caller data must also document its
register-retention cleanup strategy. The current prototypes only construct and
store zero vectors before scalar-equivalent writes; real vectorized
implementations must explain whether target-specific cleanup such as
`vzeroupper`, explicit zero registers, or another architecture-appropriate
sequence is required before returning.
