# Unsafe Code Inventory

`base64-ng` keeps scalar encode/decode in safe Rust. The crate root uses
`#![deny(unsafe_code)]`, and reviewed `allow(unsafe_code)` exceptions are
limited to volatile wipe helpers in `src/lib.rs` and the SIMD boundary in
`src/simd.rs`.

This inventory is intentionally small and release-gate enforced. Any new unsafe
block must be added here before an accelerated backend can be admitted.

## Policy

- Default builds compile audited unsafe volatile wipe helpers.
- Optional SIMD prototypes live only in `src/simd.rs`.
- `scripts/validate-unsafe-boundary.sh` fails if `allow(unsafe_code)` appears
  outside the volatile wipe helpers or `src/simd.rs`.
- `scripts/validate-unsafe-boundary.sh` fails if architecture intrinsics, CPU
  feature detection, or `target_feature` gates appear outside `src/simd.rs`.
- Every unsafe function and unsafe block must have a local safety explanation.
- Prototype functions are not eligible for runtime dispatch.

## Current Unsafe Sites

### `wipe_bytes`

Location: `src/lib.rs`

Status: active cleanup primitive.

Purpose:

- Clear initialized caller-owned and crate-owned buffers used by clear-tail,
  stream cleanup, stack-buffer cleanup, and secret-buffer cleanup APIs.
- Use volatile writes so the compiler must retain the cleanup writes even when
  the memory is not read again before drop or reuse.

Preconditions:

- Caller must pass a valid mutable byte slice.

Unsafe operation:

- `core::ptr::write_volatile` writes zero to each byte in the slice.

Safety argument:

- Each pointer is derived from a unique `&mut [u8]` iterator item.
- Each pointer is valid, aligned, non-null, and writable for exactly one `u8`.
- The helper writes only within the provided slice and does not read through the
  volatile pointer.
- A `SeqCst` compiler fence follows the volatile write loop to prevent
  reordering around the cleanup boundary.

Limitations:

- This is best-effort data-retention reduction, not a formal zeroization
  guarantee. It cannot clear historical copies, allocator spare capacity, swap,
  core dumps, CPU registers, or buffers outside the slice provided to the API.

### `wipe_vec_spare_capacity`

Location: `src/lib.rs`

Status: active cleanup primitive when `alloc` is enabled.

Purpose:

- Clear vector spare capacity for `SecretBuffer` so previously written bytes in
  the same allocation are not left untouched when the wrapper is created,
  dropped, or explicitly cleared.
- Keep spare-capacity cleanup dependency-free while still using volatile writes.

Preconditions:

- Caller must pass a valid mutable `Vec<u8>`.

Unsafe operation:

- `core::ptr::write_volatile` writes zero to each byte from `len` up to
  `capacity`.
- `ptr.add(offset)` computes a pointer inside the vector allocation's spare
  capacity.

Safety argument:

- The loop writes only while `offset < capacity`, so each computed pointer is
  inside the vector allocation.
- A `Vec<u8>` allocation is valid and aligned for `u8` writes across its full
  capacity.
- The helper does not read uninitialized spare-capacity bytes; it only writes
  zeros.
- A `SeqCst` compiler fence follows the volatile write loop to prevent
  reordering around the cleanup boundary.

Limitations:

- This is best-effort data-retention reduction, not a formal zeroization
  guarantee. It cannot make claims about allocator internals, historical copies,
  swap, core dumps, CPU registers, or buffers outside the vector allocation.

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

### `encode_12_bytes_ssse3_sse41`

Location: `src/simd.rs`

Status: inactive prototype, not dispatchable.

Purpose:

- Exercise lower-tier x86 target-feature plumbing.
- Validate the unsafe boundary.
- Provide scalar-equivalence test coverage before any real vector path is
  admitted.

Preconditions:

- Caller must prove SSSE3 and SSE4.1 are available on the current CPU.
- Input is exactly 12 bytes.
- Output is exactly 16 bytes.

Unsafe operation:

- `_mm_storeu_si128` stores one 128-bit zero vector into the output buffer.

Safety argument:

- The output type is `&mut [u8; 16]`, so the store has enough initialized,
  writable memory.
- The intrinsic is the unaligned store variant, so no stronger alignment is
  required.
- The function is guarded by an SSSE3/SSE4.1 target-feature contract.
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

The admission bar applies equally to AVX2, AVX-512, SSSE3/SSE4.1, NEON, wasm
`simd128`, and any other future vector backend.

Any admitted SIMD path that processes caller data must also document its
register-retention cleanup strategy. The current prototypes only construct and
store zero vectors before scalar-equivalent writes; real vectorized
implementations must explain whether target-specific cleanup such as
`vzeroupper`, explicit zero registers, or another architecture-appropriate
sequence is required before returning.
