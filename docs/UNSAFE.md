# Unsafe Code Inventory

`base64-ng` keeps scalar encode/decode in safe Rust. The crate root uses
`#![deny(unsafe_code)]`, and reviewed `allow(unsafe_code)` exceptions are
limited to volatile wipe helpers, the constant-time comparison accumulator
barrier, the validated secret UTF-8 conversion helper, the constant-time error
gate barrier in `src/lib.rs`, and the SIMD boundary in `src/simd.rs`.

This inventory is intentionally small and release-gate enforced. Any new unsafe
block must be added here before an accelerated backend can be admitted.

## Policy

- Default builds compile audited unsafe volatile wipe helpers, the
  constant-time comparison accumulator barrier, the validated secret UTF-8
  conversion helper, and the constant-time error gate barrier.
- Optional SIMD prototypes live only in `src/simd.rs` and are compiled only
  for tests until a real backend is admitted.
- `scripts/validate-unsafe-boundary.sh` fails if `allow(unsafe_code)` appears
  outside the volatile wipe helpers, the constant-time comparison accumulator
  barrier, the validated secret UTF-8 conversion helper, the constant-time
  error gate barrier, or `src/simd.rs`.
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
- Keep the wipe loop behind an `#[inline(never)]` call boundary so release and
  LTO builds have less surrounding caller context when optimizing cleanup.

Preconditions:

- Caller must pass a valid mutable byte slice.

Unsafe operation:

- `core::ptr::write_volatile` writes zero to each byte in the slice.
- `wipe_barrier` receives the slice pointer and length after the volatile
  writes and, on supported native architectures, passes them through a
  `core::arch::asm!` block as opaque inputs while also issuing a store-ordering
  fence before the final compiler fence.

Safety argument:

- Each pointer is derived from a unique `&mut [u8]` iterator item.
- Each pointer is valid, aligned, non-null, and writable for exactly one `u8`.
- The helper writes only within the provided slice and does not read through the
  volatile pointer.
- The barrier does not dereference the pointer. It exists to keep the preceding
  volatile writes visible across a cleanup boundary, including under more
  aggressive optimization, and to order the issued zero stores on supported
  native architectures before a `SeqCst` compiler fence.
- `wipe_bytes` and `wipe_barrier` are both `#[inline(never)]` to preserve
  explicit cleanup call boundaries for generated-code review.

Limitations:

- This is best-effort data-retention reduction, not a formal zeroization
  guarantee. The inline assembly barrier strengthens the optimizer boundary and
  orders stores on supported native architectures, but it cannot clear
  historical copies, compiler spill slots, allocator spare capacity, swap,
  hibernation images, core dumps, CPU registers, cache lines, write buffers,
  cold-boot remanence, or buffers outside the slice provided to the API.
  Software-only wiping also cannot make claims about temporary stack copies
  created before the wipe boundary. Miri, `wasm32`, and unsupported native
  architectures fall back to the compiler fence only. On `wasm32`, downstream
  runtime JIT behavior is outside this crate's control; `wasm32` builds
  therefore fail closed unless `allow-wasm32-best-effort-wipe` is explicitly
  enabled. Unsupported native architectures also fail closed unless
  `allow-compiler-fence-only-wipe` is explicitly enabled after platform review.
- Callers with platform-specific formal zeroization requirements should apply
  their own zeroization policy to caller-owned buffers in addition to using the
  crate cleanup APIs. Applications that already admit dependencies such as
  `zeroize` may combine them with `base64-ng` caller-owned buffers after the
  Base64 operation.
  High-assurance deployments should also use OS controls such as locked memory
  where available, disabled or encrypted swap and hibernation, crash-dump
  suppression, short key lifetimes, and allocator isolation for secret regions.

### `wipe_barrier`

Location: `src/lib.rs`

Status: active cleanup-boundary hardening primitive.

Purpose:

- Keep volatile wipe writes observable across a cleanup boundary without adding
  a runtime dependency.
- On supported native architectures, provide a stable inline assembly optimizer
  barrier and store-ordering fence similar in shape to dependency-backed
  zeroization crates.
- Fall back to a `SeqCst` compiler fence under Miri and on architectures where
  the crate does not enable inline assembly.

Preconditions:

- Caller passes a pointer and length describing the region that was just wiped.
- The function does not dereference the pointer, so empty or dangling
  zero-length slice pointers are accepted as opaque optimizer inputs.

Unsafe operation:

- `core::arch::asm!` emits `mfence` on non-Miri `x86`/`x86_64`,
  `dsb sy; isb sy` on non-Miri `arm`, `dsb sy; isb sy; hint #20` on non-Miri
  `aarch64`, and `fence rw, rw` on non-Miri `riscv32`/`riscv64`. The pointer
  and length are also passed as opaque operands.

Safety argument:

- The assembly block does not access memory through the pointer.
- `options(nostack, preserves_flags)` states that the block does not use the
  stack or modify flags.
- Pointer and length operands are used only as opaque inputs to prevent the
  optimizer from reasoning away the preceding volatile writes.

Limitations:

- This is an optimizer and store-ordering barrier, not a hardware erasure
  primitive. It does not clear registers, cache lines, write buffers, stack
  spills, swap, hibernation images, core dumps, cold-boot remanence, or
  historical copies.
- It does not upgrade `wipe_bytes` or `wipe_vec_spare_capacity` to a formal
  zeroization guarantee.
- `wasm32` currently uses only the final compiler fence. Wasm runtime JITs may
  apply additional optimizations or retain memory outside the Rust compiler
  boundary. `wasm32` builds therefore fail closed unless
  `allow-wasm32-best-effort-wipe` is explicitly enabled.
- Unsupported native architectures currently use only the final compiler fence.
  They fail closed unless `allow-compiler-fence-only-wipe` is explicitly
  enabled after reviewing this weaker cleanup posture and applying platform
  memory controls.
- On RISC-V, `fence rw, rw` is a store-ordering fence for wipe cleanup. It is
  reported separately from the constant-time result gate posture and should not
  be read as a Spectre-v1 speculation isolation guarantee.

### `constant_time_eq_same_len`

Location: `src/lib.rs`

Status: active constant-time-oriented comparison primitive.

Purpose:

- Compare equal-length redacted buffer contents without short-circuiting on the
  first differing byte.
- Keep the byte-difference accumulator observable to the optimizer after each
  iteration before the public equality result is reported.

Preconditions:

- Callers must pass slices with the same public length. The public-length
  wrapper checks this before calling the helper.

Unsafe operation:

- `core::ptr::read_volatile` reads the initialized local `diff` accumulator
  after each OR reduction.

Safety argument:

- `diff` is an initialized stack-local `u8` for the entire loop.
- The volatile read does not read from caller memory and cannot violate bounds
  or aliasing requirements.
- The helper is `#[inline(never)]` and also passes the final accumulator
  through `ct_error_gate_barrier` before returning the public equality result.

Limitations:

- This is dependency-free defense in depth against optimizer rewrites, not a
  formal cryptographic comparison guarantee. Applications that require an
  audited MAC, token, or password-hash comparison primitive should use one at
  the application boundary.

### `string_from_validated_secret_bytes`

Location: `src/lib.rs`

Status: active secret conversion helper when `alloc` is enabled.

Purpose:

- Convert bytes already validated as UTF-8, or bytes just extracted from a
  valid `String`, back into `String` without a second fallible conversion after
  the vector has crossed a secret-wrapper boundary.
- Avoid an intermediate raw `Vec<u8>` drop path that would not run the crate's
  best-effort cleanup on panic.

Preconditions:

- Caller must validate the exact vector contents as UTF-8 immediately before
  handing ownership to this helper, or pass bytes produced directly by
  `String::into_bytes` without modifying the initialized prefix.
- The vector must not be modified between validation and conversion.

Unsafe operation:

- `alloc::string::String::from_utf8_unchecked` converts the vector to a
  string without revalidating.

Safety argument:

- `SecretBuffer::try_into_exposed_string` validates `self.expose_secret()` with
  `core::str::from_utf8` before moving the same allocation into this helper.
- `ExposedSecretString::from_string` receives a valid `String`, converts it to
  bytes, wipes only spare capacity past the initialized UTF-8 prefix, and then
  moves the same initialized prefix into this helper.
- No mutation occurs between validation and conversion.
- The returned `ExposedSecretString` retains redacted formatting and drop-time
  cleanup.

### `ct_error_gate_barrier`

Location: `src/lib.rs`

Status: active constant-time error-gate hardening primitive.

Purpose:

- Keep the accumulated constant-time decoder malformed-input mask visible
  across a non-inlined boundary before the public success/failure branch.
- Emit an architecture-specific speculation or ordering barrier where stable
  Rust supports one locally.

Preconditions:

- Caller passes accumulated public error-mask bytes.

Unsafe operation:

- `core::arch::asm!` emits `lfence` on non-Miri `x86`/`x86_64`, `isb sy` on
  non-Miri 32-bit `arm`, `isb sy; hint #20` on non-Miri `aarch64`, and
  `fence rw, rw` on non-Miri `riscv32`/`riscv64`.

Safety argument:

- The assembly blocks do not access memory.
- `options(nostack, preserves_flags)` states that the blocks do not use the
  stack or modify flags. The x86/x86_64 block also uses `nomem`.
- The helper does not read or write through any pointer and cannot violate
  Rust aliasing or bounds rules.

Limitations:

- This is defense in depth against speculation around the final public
  malformed-input result. It does not make the ct decoder a formally verified
  hardware side-channel resistant primitive.
- 32-bit ARM uses `isb sy` without CSDB, and RISC-V base ISA has no canonical
  speculation barrier. The crate reports both CT gate postures as
  `ordering-fence` rather than `hardware-speculation-barrier`.
- On AArch64, the CSDB hint may be treated as a no-op on older cores. The
  runtime posture reports the emitted barrier sequence, not a formal
  microarchitecture certification.
- Unsupported architectures fall back to the compiler fence only.

### `ct_decode_alphabet_byte`

Location: `src/lib.rs`

Status: active constant-time-oriented alphabet scanner.

Purpose:

- Decode one Base64 symbol by scanning all 64 alphabet entries instead of
  indexing a decode table or returning at the first match.
- Keep the decoded-value and validity accumulators observable to the optimizer
  on every iteration of the fixed scan.

Preconditions:

- `A::ENCODE` is a validated 64-byte Base64 alphabet. Built-in alphabets and
  the `define_alphabet!` macro enforce this.

Unsafe operation:

- `core::ptr::read_volatile` reads initialized local `decoded` and `valid`
  accumulators after each OR reduction.

Safety argument:

- `decoded` and `valid` are initialized stack-local `u8` values for the entire
  loop.
- The volatile reads do not read from caller memory and cannot violate bounds
  or aliasing requirements.
- The function remains `#[inline(never)]` so generated-code review can inspect
  the scanner as a distinct helper.

Limitations:

- These volatile reads are optimizer barriers, not a formal proof of
  microarchitectural constant-time behavior. Release evidence and dudect remain
  required for high-assurance review.

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
- `wipe_barrier` is called for the spare-capacity region after the volatile
  write loop.

Safety argument:

- The loop writes only while `offset < capacity`, so each computed pointer is
  inside the vector allocation.
- The helper returns before computing the barrier pointer when spare capacity
  is zero. This avoids passing a one-past-the-end allocation pointer or a
  dangling zero-capacity vector sentinel to the barrier.
- A `Vec<u8>` allocation is valid and aligned for `u8` writes across its full
  capacity.
- The helper does not read uninitialized spare-capacity bytes; it only writes
  zeros.
- When spare capacity is non-zero, the barrier pointer is computed with
  `ptr.add(len)`, which points inside the vector allocation at the first spare
  byte. The barrier does not dereference the pointer. It exists to keep the
  preceding volatile writes visible across the cleanup boundary before the
  final `SeqCst` compiler fence.

Limitations:

- This is best-effort data-retention reduction, not a formal zeroization
  guarantee. It cannot make claims about allocator internals, historical
  copies, compiler spill slots, swap, core dumps, CPU registers, or buffers
  outside the vector allocation. Applications with a platform-specific
  zeroization policy should still apply that policy at the ownership boundary.

### `encode_48_bytes_avx512`

Location: `src/simd.rs`

Status: inactive test-only prototype, not compiled into release library builds
and not dispatchable.

Purpose:

- Exercise AVX-512 target-feature plumbing.
- Validate the unsafe boundary before an admitted AVX-512 path exists.
- Provide scalar-equivalence scaffolding before any real vector path is
  admitted. Current tests do not prove vectorized Base64 correctness.

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
  The SIMD zeroing is semantically overwritten and is not an implementation of
  vectorized Base64.
- Register-retention note: this prototype does not load caller bytes into SIMD
  registers. Any future AVX-512 implementation that does so must document and
  implement explicit cleanup for every secret-bearing ZMM/YMM/XMM register
  before return, plus AVX transition cleanup such as `vzeroupper` where
  applicable.

### `encode_24_bytes_avx2`

Location: `src/simd.rs`

Status: inactive test-only prototype, not compiled into release library builds
and not dispatchable.

Purpose:

- Exercise AVX2 target-feature plumbing.
- Validate the unsafe boundary.
- Provide scalar-equivalence scaffolding before any real vector path is
  admitted. Current tests do not prove vectorized Base64 correctness.

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
  The SIMD zeroing is semantically overwritten and is not an implementation of
  vectorized Base64.
- Register-retention note: this prototype does not load caller bytes into SIMD
  registers. Any future AVX2 implementation that does so must document and
  implement explicit cleanup for every secret-bearing YMM/XMM register before
  return, plus AVX transition cleanup such as `vzeroupper` where applicable.

### `encode_12_bytes_ssse3_sse41`

Location: `src/simd.rs`

Status: inactive test-only prototype, not compiled into release library builds
and not dispatchable.

Purpose:

- Exercise lower-tier x86 target-feature plumbing.
- Validate the unsafe boundary.
- Provide scalar-equivalence scaffolding before any real vector path is
  admitted. Current tests do not prove vectorized Base64 correctness.

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
  The SIMD zeroing is semantically overwritten and is not an implementation of
  vectorized Base64.
- Register-retention note: this prototype does not load caller bytes into SIMD
  registers. Any future SSSE3/SSE4.1 implementation that does so must document
  and implement explicit cleanup for every secret-bearing XMM register before
  return.

### `encode_12_bytes_neon`

Location: `src/simd.rs`

Status: inactive test-only prototype, not compiled into release library builds
and not dispatchable.

Purpose:

- Exercise ARM NEON intrinsic plumbing.
- Validate the unsafe boundary on ARM targets.
- Provide scalar-equivalence scaffolding before any real vector path is
  admitted. Current tests do not prove vectorized Base64 correctness.

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
  The NEON zeroing is semantically overwritten and is not an implementation of
  vectorized Base64.
- Register-retention note: this prototype does not load caller bytes into SIMD
  registers. Any future NEON implementation that does so must document and
  implement explicit cleanup for every secret-bearing V/Q register before
  return.

## Admission Rule

Unsafe SIMD can become an active backend only after scalar differential tests,
fuzz evidence, architecture-specific build evidence, benchmark evidence, and
review of this inventory all pass for that release.
Future SIMD dispatch work must also complete
[SIMD_ACTIVATION_CHECKLIST.md](SIMD_ACTIVATION_CHECKLIST.md).

The admission bar applies equally to AVX2, AVX-512, SSSE3/SSE4.1, NEON, wasm
`simd128`, and any other future vector backend.

Any admitted SIMD path that processes caller data must also document its
register-retention cleanup strategy and include the matching explicit register
cleanup implementation, generated-assembly evidence, and tests in the admission
evidence. This is a hard release blocker before dispatch, not an optional
follow-up. The current prototypes only construct and store zero vectors before
scalar-equivalent writes; the exemption ends as soon as a prototype loads
caller bytes into vector registers.
