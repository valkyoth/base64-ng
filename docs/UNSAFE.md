# Unsafe Code Inventory

`base64-ng` keeps scalar encode/decode in safe Rust. The crate root uses
`#![deny(unsafe_code)]`, and reviewed `allow(unsafe_code)` exceptions are
limited to volatile wipe helpers in `src/cleanup.rs`, the constant-time
comparison accumulator barrier and constant-time error gate barrier in
`src/ct/`, and the SIMD boundary in `src/simd/`.

This inventory is intentionally small and release-gate enforced. Any new unsafe
block must be added here before an accelerated backend can be admitted.

## Policy

- Default builds compile audited unsafe volatile wipe helpers, the
  constant-time comparison accumulator barrier, and the constant-time error
  gate barrier.
- Optional SIMD code lives only in `src/simd/`. Admitted encode and strict
  decode backends are reachable only through the reviewed runtime dispatch
  boundaries named in `docs/SIMD_ADMISSION.md`; prototype-only backends remain
  test/evidence code and are not eligible for runtime dispatch.
- `scripts/validate-unsafe-boundary.sh` fails if `allow(unsafe_code)` appears
  outside `src/cleanup.rs`, `src/ct/`, or `src/simd/`.
- `scripts/validate-unsafe-boundary.sh` fails if architecture intrinsics, CPU
  feature detection, or `target_feature` gates appear outside the reviewed
  cleanup, constant-time gate, and SIMD boundaries.
- Every unsafe function and unsafe block must have a local safety explanation.
- Prototype functions are not eligible for runtime dispatch.

## Current Unsafe Sites

### `wipe_bytes`

Location: `src/cleanup.rs`

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
  created before the wipe boundary. Miri, Kani, `wasm32`, and unsupported
  native architectures fall back to the compiler fence only. The Miri and Kani
  fallbacks are verifier/interpreter constraints, not deployed runtime
  postures. On `wasm32`, downstream runtime JIT behavior is outside this
  crate's control; `wasm32` builds therefore fail closed unless
  `allow-wasm32-best-effort-wipe` is explicitly enabled. Unsupported native
  architectures also fail closed unless `allow-compiler-fence-only-wipe` is
  explicitly enabled after platform review.
- Callers with platform-specific formal zeroization requirements should apply
  their own zeroization policy to caller-owned buffers in addition to using the
  crate cleanup APIs. Applications that already admit dependencies such as
  `zeroize` may combine them with `base64-ng` caller-owned buffers after the
  Base64 operation.
  High-assurance deployments should also use OS controls such as locked memory
  where available, disabled or encrypted swap and hibernation, crash-dump
  suppression, short key lifetimes, and allocator isolation for secret regions.

### `wipe_barrier`

Location: `src/cleanup.rs`

Status: active cleanup-boundary hardening primitive.

Purpose:

- Keep volatile wipe writes observable across a cleanup boundary without adding
  a runtime dependency.
- On supported native architectures, provide a stable inline assembly optimizer
  barrier and store-ordering fence similar in shape to dependency-backed
  zeroization crates.
- Fall back to a `SeqCst` compiler fence under Miri, under Kani, and on
  architectures where the crate does not enable inline assembly.

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

Location: `src/ct/`

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

### `ct_accumulate_u8`

Location: `src/ct/`

Status: active constant-time-oriented accumulator hardening primitive.

Purpose:

- Accumulate byte masks and decoded-byte state through a non-inlined helper so
  comparison and alphabet-scan loops do not expose the OR reduction as a simple
  in-loop optimizer pattern.
- Keep each accumulator update observable through a volatile read.

Preconditions:

- Caller passes initialized `u8` values.

Unsafe operation:

- `core::ptr::read_volatile` reads the initialized local `result` accumulator.

Safety argument:

- `result` is an initialized stack-local `u8`.
- The volatile read does not read from caller memory and cannot violate bounds
  or aliasing requirements.
- The helper is `#[inline(never)]`; callers use it only for local byte-mask
  accumulation in constant-time-oriented helpers.

Limitations:

- This strengthens the optimizer boundary but is still dependency-free,
  best-effort hardening rather than a formal machine-code constant-time proof.

### `ct_error_gate_barrier`

Location: `src/ct/`

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
  runtime posture reports `hardware-speculation-barrier-unattested`, not a
  formal microarchitecture certification or a passing
  `HighAssuranceScalarOnly` posture.
- On AArch64, `base64_ng_aarch64_csdb_attested` is an operator attestation cfg.
  It changes the runtime posture only after the deployment has independently
  verified that the target core treats CSDB as an effective speculation
  barrier. The reported posture is
  `hardware-speculation-barrier-build-asserted`, not the generic native
  `hardware-speculation-barrier`, so audit logs retain the evidence boundary.
  It is intentionally not a Cargo feature, so `--all-features` cannot enable it
  accidentally.
- Unsupported architectures fall back to the compiler fence only.

### `ct_decode_alphabet_byte`

Location: `src/ct/`

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
- `#[inline(never)]` is supported by generated-code evidence, not by a
  language-level formal guarantee under all future LTO optimizers. The release
  evidence script checks that this scanner remains a separate text symbol in
  the LTO artifact; high-assurance deployments should keep that evidence check
  in their release gate.

### `wipe_vec_spare_capacity`

Location: `src/cleanup.rs`

Status: active cleanup primitive when `alloc` is enabled.

Purpose:

- Clear vector spare capacity for `SecretBuffer` so previously written bytes in
  the same allocation are not left untouched when the wrapper is created,
  dropped, or explicitly cleared.
- Keep spare-capacity cleanup dependency-free while still using volatile writes.

Preconditions:

- Caller must pass a valid mutable `Vec<u8>`.

Unsafe operation:

- `Vec::spare_capacity_mut` exposes the uninitialized spare allocation as
  `&mut [MaybeUninit<u8>]`.
- `core::ptr::write_volatile` writes zero to each spare-capacity byte through
  the slot's `MaybeUninit<u8>::as_mut_ptr`.
- `wipe_barrier` is called for the spare-capacity region after the volatile
  write loop.

Safety argument:

- `spare_capacity_mut` only returns slots inside the vector allocation after
  the initialized length.
- The helper returns before computing the barrier pointer when spare capacity
  is zero. This avoids passing a dangling zero-capacity vector sentinel to the
  barrier.
- The helper does not read uninitialized spare-capacity bytes; it only writes
  zeros.
- When spare capacity is non-zero, the barrier pointer is the start of the
  spare-capacity slice. The barrier does not dereference the pointer. It exists
  to keep the preceding volatile writes visible across the cleanup boundary
  before the final `SeqCst` compiler fence.

Limitations:

- This is best-effort data-retention reduction, not a formal zeroization
  guarantee. It cannot make claims about allocator internals, historical
  copies, compiler spill slots, swap, core dumps, CPU registers, or buffers
  outside the vector allocation. Applications with a platform-specific
  zeroization policy should still apply that policy at the ownership boundary.

### `encode_48_bytes_avx512`

Location: `src/simd/x86/mod.rs`

Status: admitted std x86/x86_64 AVX-512 VBMI encode block for Standard and
URL-safe alphabet families. It is reachable through runtime-probed AVX-512 VBMI
encode dispatch for fixed 48-byte blocks. Unsupported alphabets, tails,
padding, `no_std`, in-place encode, and decode use scalar fallback.

Purpose:

- Exercise AVX-512 target-feature plumbing.
- Provide the fixed-block vector encode primitive for the admitted AVX-512 VBMI
  encode backend.

Preconditions:

- Caller must prove the full AVX-512 Base64 candidate bundle is available on
  the current CPU: `avx512f`, `avx512bw`, `avx512vl`, and `avx512vbmi`.
- Input is exactly 48 bytes.
- Output is exactly 64 bytes.

Unsafe operation:

- `_mm512_loadu_si512` loads from a local 64-byte staging array that contains
  four 12-byte input lanes plus four zero bytes per lane.
- `_mm512_loadu_si512` loads a fixed shuffle mask and the 64-byte alphabet
  table.
- `_mm512_shuffle_epi8` reshapes each staged 128-bit lane into four 24-bit
  groups without reading from caller memory beyond the fixed input array.
- AVX-512 shifts, masks, and OR operations produce sixty-four 6-bit indices.
- `_mm512_permutexvar_epi8` uses the VBMI byte-permute instruction to map those
  indices through the loaded alphabet table.
- `_mm512_storeu_si512` stores the 64 encoded bytes into the output buffer.
- `clear_zmm_registers_after_encode_block` clears ZMM state and uses
  `vzeroupper` before return to reduce register retention and AVX/SSE
  transition state in this encode block.
- The local staging array is wiped with the crate cleanup primitive before the
  function returns.

Safety argument:

- The input and output array types provide fixed readable and writable bounds.
- The SIMD load reads only from a local 64-byte staging array, so the encoder
  does not over-read the 48-byte caller input.
- The staging array is mutable and wiped after the SIMD store and register
  cleanup, reducing stack retention of the copied caller bytes.
- The load and store intrinsics are unaligned variants, so no stronger
  alignment is required.
- The function is guarded by the full AVX-512 Base64 target-feature contract.
- The index vector is masked to `0..=63` before the VBMI table lookup.
- The output length is fixed by the output array type.
- Runtime dispatch reaches this block only after `std` runtime CPU probing
  proves the full AVX-512 VBMI feature bundle. Direct tests call it only after
  the same feature check.
- Register-retention note: the encoder loads caller bytes into ZMM state. It
  calls `clear_zmm_registers_after_encode_block` before return. This is
  retention reduction for the admitted encode block, not a formal
  microarchitectural side-channel proof.

### `clear_zmm_registers_after_encode_block`

Location: `src/simd/x86/cleanup.rs`

Status: private helper for the admitted AVX-512 VBMI encode block and its tests.

Purpose:

- Clear ZMM state before returning from the AVX-512 encode block that processes
  caller bytes in vector registers.

Preconditions:

- Called only after the encode block has stored its output and no later AVX-512
  value is needed by the function.

Unsafe operation:

- Inline assembly zeros the ZMM register set available to the target (`zmm0`
  through `zmm7` on `x86`, `zmm0` through `zmm31` on `x86_64`) and declares
  those registers as clobbered outputs.
- Inline assembly emits `vzeroupper` to clear upper vector state before
  returning to scalar/SSE code.

Safety argument:

- The helper does not read or write memory.
- The helper runs at the end of the AVX-512 encode block path.
- Clobbered registers are declared to the compiler with explicit `out("zmmN")`
  operands.
- This is best-effort register-retention reduction for encode evidence, not a
  guarantee that historical register, stack, cache, or microarchitectural
  copies do not exist.

### `encode_24_bytes_avx2`

Location: `src/simd/x86/mod.rs`

Status: admitted std x86/x86_64 AVX2 encode block for Standard and URL-safe
alphabet families. It is reachable through runtime-probed AVX2 encode dispatch
and test evidence.

Purpose:

- Exercise AVX2 target-feature plumbing.
- Validate the unsafe boundary.
- Provide the fixed-block vector encode primitive for the admitted AVX2 encode
  backend.

Preconditions:

- Caller must prove AVX2 is available on the current CPU.
- Input is exactly 24 bytes.
- Output is exactly 32 bytes.
- The vectorized path is used only for Standard-family alphabets (`A-Z`,
  `a-z`, `0-9`, and either `+/` or `-_`). Other alphabets fall back to the
  scalar prototype loop.

Unsafe operation:

- `_mm256_loadu_si256` loads from a local 32-byte staging array that contains
  two 12-byte input lanes plus four zero bytes per lane.
- `_mm256_shuffle_epi8` reshapes each staged 128-bit lane into four 24-bit
  groups without reading from caller memory beyond the fixed input array.
- AVX2 shifts, masks, and OR operations produce thirty-two 6-bit indices.
- `encode_standard_family_indices_avx2` maps those indices to Standard or
  URL-safe alphabet bytes with AVX2 byte blends.
- `_mm256_storeu_si256` stores the 32 encoded bytes into the output buffer.
- `clear_ymm_registers_after_encode_block` clears XMM lower halves and uses
  `vzeroupper` before return to reduce register retention and AVX/SSE
  transition state in this vector encode path.
- The local staging array is wiped with the crate cleanup primitive before the
  function returns.

Safety argument:

- The input and output array types provide fixed readable and writable bounds.
- The SIMD load reads only from a local 32-byte staging array, so the prototype
  does not over-read the 24-byte caller input.
- The staging array is mutable and wiped after the SIMD store and register
  cleanup, reducing stack retention of the copied caller bytes in this inactive
  prototype.
- The load and store intrinsics are unaligned variants, so no stronger
  alignment is required.
- The function is guarded by an AVX2 target-feature contract.
- The output length is fixed by the output array type.
- Runtime dispatch is gated by `std::is_x86_feature_detected!` and the
  Standard-family alphabet check; unsupported CPUs, custom alphabets, `no_std`,
  in-place encode, line-ending insertion, and decode use scalar fallback.
  Final tail and padding completion use scalar code.
- Register-retention note: the path loads caller bytes into YMM/XMM state. It
  calls `clear_ymm_registers_after_encode_block` before return. This is
  retention reduction, not a formal microarchitectural side-channel proof.

### `encode_standard_family_indices_avx2`

Location: `src/simd/x86/mod.rs`

Status: private helper for the admitted AVX2 encode block and its tests.

Purpose:

- Map thirty-two 6-bit indices to Standard or URL-safe alphabet bytes with AVX2
  blends instead of scalar per-byte table indexing.

Preconditions:

- Caller must prove AVX2 is available on the current CPU.
- `indices` contains only byte values in `0..=63`.
- The alphabet must be Standard-family as checked by the caller with a complete
  comparison of positions `0..62` and an explicit check of the two terminal
  symbols.

Unsafe operation:

- AVX2 byte comparisons and blends compute the ASCII offset for each index.

Safety argument:

- The helper does not dereference raw pointers or access memory.
- The target-feature contract enables the required AVX2 instructions.
- The caller constructs `indices` with masks that constrain every byte to a
  six-bit Base64 value.
- The helper is private to the Standard-family AVX2 encode path.

### `clear_ymm_registers_after_encode_block`

Location: `src/simd/x86/cleanup.rs`

Status: private helper for admitted AVX2 encode and inactive x86 prototype
tests.

Purpose:

- Clear lower XMM state and upper YMM state before returning from the AVX2
  encode path that processes caller bytes in vector registers.

Preconditions:

- Called only after the prototype has stored its output and no later AVX/SSE
  value is needed by the function.

Unsafe operation:

- Calls `clear_xmm_registers_after_encode_block` for lower XMM register state.
- Inline assembly emits `vzeroupper` to clear upper YMM state and avoid
  carrying AVX upper halves back to scalar/SSE code.

Safety argument:

- The helper does not read or write memory.
- The helper runs at the end of the AVX2 encode block path.
- `vzeroupper` is valid under the AVX2 target-feature precondition inherited
  from the caller.
- This is best-effort register-retention reduction for test evidence, not a
  guarantee that historical register, stack, cache, or microarchitectural
  copies do not exist.

### `encode_12_bytes_ssse3_sse41`

Location: `src/simd/x86/mod.rs`

Status: admitted std x86/x86_64 SSSE3/SSE4.1 encode block for Standard and
URL-safe alphabet families. It is reachable through runtime-probed SSSE3/SSE4.1
encode dispatch and test evidence.

Purpose:

- Exercise lower-tier x86 target-feature plumbing.
- Validate the unsafe boundary.
- Provide the fixed-block vector encode primitive for the admitted SSSE3/SSE4.1
  encode backend.

Preconditions:

- Caller must prove SSSE3 and SSE4.1 are available on the current CPU.
- Input is exactly 12 bytes.
- Output is exactly 16 bytes.
- The vectorized path is used only for Standard-family alphabets (`A-Z`,
  `a-z`, `0-9`, and either `+/` or `-_`). Other alphabets fall back to the
  scalar prototype loop.

Unsafe operation:

- `_mm_loadu_si128` loads from a local 16-byte staging array that contains the
  12-byte input plus four zero bytes.
- `_mm_shuffle_epi8` reshapes the staged bytes into four 24-bit lanes without
  reading from caller memory beyond the fixed input array.
- SSE2 shifts, masks, and OR operations produce sixteen 6-bit indices.
- `encode_standard_family_indices_ssse3_sse41` maps those indices to Standard
  or URL-safe alphabet bytes with SSE4.1 byte blends.
- `_mm_storeu_si128` stores the 16 encoded bytes into the output buffer.
- `clear_xmm_registers_after_encode_block` clears XMM registers before return
  to reduce register retention in the vector encode path.
- The local staging array is wiped with the crate cleanup primitive before the
  function returns.

Safety argument:

- The input and output array types provide fixed readable and writable bounds.
- The SIMD load reads only from a local 16-byte staging array, so the prototype
  does not over-read the 12-byte caller input.
- The staging array is mutable and wiped after the SIMD store and XMM cleanup,
  reducing stack retention of the copied caller bytes.
- The load and store intrinsics are unaligned variants, so no stronger
  alignment is required.
- The function is guarded by an SSSE3/SSE4.1 target-feature contract.
- The output length is fixed by the output array type.
- Runtime dispatch is gated by `std::is_x86_feature_detected!` and the
  Standard-family alphabet check; unsupported CPUs, custom alphabets, `no_std`,
  in-place encode, line-ending insertion, and decode use scalar fallback.
  Final tail and padding completion use scalar code.
- Register-retention note: the path loads caller bytes into XMM registers. It
  calls `clear_xmm_registers_after_encode_block` before return. This is
  retention reduction, not a formal microarchitectural side-channel proof.

### `encode_standard_family_indices_ssse3_sse41`

Location: `src/simd/x86/mod.rs`

Status: private helper for the admitted SSSE3/SSE4.1 encode block and its
tests.

Purpose:

- Map sixteen 6-bit indices to Standard or URL-safe alphabet bytes with SSE4.1
  blends instead of scalar per-byte table indexing.

Preconditions:

- Caller must prove SSE4.1 is available on the current CPU.
- `indices` contains only byte values in `0..=63`.
- The alphabet must be Standard-family as checked by the caller with a complete
  comparison of positions `0..62` and an explicit check of the two terminal
  symbols.

Unsafe operation:

- SSE4.1 byte comparisons and blends compute the ASCII offset for each index.

Safety argument:

- The helper does not dereference raw pointers or access memory.
- The target-feature contract enables the required SSE4.1 instructions.
- The caller constructs `indices` with masks that constrain every byte to a
  six-bit Base64 value.
- The helper is private to the Standard-family SSSE3/SSE4.1 encode path.

### `decode_slice_ssse3_sse41`, `decode_slice_avx2`, and `decode_slice_avx512`

Location: `src/simd/x86/decode.rs`

Status: admitted std x86/x86_64 strict decode dispatch wrappers for Standard
and URL-safe alphabet families. They are reachable only when the `simd` and
`std` features are enabled and runtime CPU feature probing has selected the
matching backend.

Purpose:

- Carve full encoded input blocks into fixed-size array references for the
  target-feature decode block functions.
- Preserve scalar public error shape by validating the complete input before
  any SIMD block output is copied to caller-visible buffers.
- Fall back from AVX-512 to AVX2, from AVX2 to SSSE3/SSE4.1, and from
  SSSE3/SSE4.1 to scalar for shorter tails or unsupported surfaces.

Preconditions:

- Runtime dispatch has selected only a backend whose CPU features are present.
- The input block loop guard proves that each carved block is fully within the
  original input slice.
- The output capacity has been checked against the scalar validated decoded
  length before any block output is copied.

Unsafe operation:

- Each wrapper uses `input.as_ptr().add(read).cast::<[u8; N]>()` and
  dereferences the result to pass a fixed-size block reference to the matching
  target-feature decoder.

Safety argument:

- `read + N <= input.len()` is checked before every raw-pointer block carve.
- `read` advances by exactly `N`, so the pointer remains within the same input
  allocation and never crosses the slice boundary.
- The wrapper never constructs a mutable alias to input memory.
- Output writes go through private stack staging buffers first. Bytes are
  copied to caller output only after whole-input scalar validation and block
  equality checks inside the target-feature decoder.
- Any unexpected block-level error wipes the local decoded staging buffer and
  rebases the error index to the original input. Tail fallback errors are also
  rebased to the original input offset.
- Unsupported alphabets, short inputs, tails, wrapped decode, legacy decode,
  in-place decode, CT secret decode, and `no_std` stay scalar. Wasm decode is
  admitted only through its separate narrow `simd128` profile.

### `decode_16_bytes_ssse3_sse41`

Location: `src/simd/x86/decode.rs`

Status: admitted std x86/x86_64 SSSE3/SSE4.1 strict decode block for Standard
and URL-safe alphabet families. It is reachable through strict decode dispatch
for full 16-byte encoded blocks after whole-input scalar validation.

Purpose:

- Provide the fixed-block SSSE3/SSE4.1 decode primitive for the admitted strict
  decode boundary without changing scalar public error behavior.
- Exercise SSSE3/SSE4.1 6-bit-value packing for a 16-byte encoded block that
  decodes to at most 12 bytes.
- Verify error agreement, padding behavior, canonical trailing-bit rejection,
  and rejected-input output retention before any later dispatch admission.

Preconditions:

- Caller must prove SSSE3 and SSE4.1 are available on the current CPU.
- Input is exactly 16 encoded bytes.
- Output is exactly 12 bytes.
- The vectorized packing path is used only for Standard-family alphabets
  (`A-Z`, `a-z`, `0-9`, and either `+/` or `-_`). Other alphabets use the
  staged scalar fallback inside the prototype.

Unsafe operation:

- `_mm_loadu_si128` loads sixteen 6-bit values from a local staging array.
- `_mm_maddubs_epi16` and `_mm_madd_epi16` pack groups of four 6-bit values
  into 24-bit decoded quanta.
- `_mm_shuffle_epi8` compacts the packed quanta into byte order.
- `_mm_storeu_si128` stores the packed prototype output into a local 16-byte
  staging array.
- `clear_xmm_registers_after_encode_block` clears XMM registers before return
  to reduce register retention in the vector decode block.

Safety argument:

- Scalar strict decode validation runs into a private 12-byte staging buffer
  before any byte is copied to the caller output. If validation fails, the
  caller output is left untouched and the staging buffer is wiped.
- The input and output array types provide fixed readable and writable bounds.
- The SIMD load and store operate only on local 16-byte arrays and use
  unaligned intrinsics, so no alignment precondition is required.
- The SSSE3/SSE4.1 target-feature contract enables every intrinsic used by
  the prototype.
- The prototype copies only the validated decoded length to caller output after
  an unconditional release-mode equality check proves the vector-packed prefix
  matches the scalar-validation prefix. It wipes local value, packed, and
  scalar-validation staging buffers before returning.
- The public dispatch wrapper validates the complete input with scalar before
  calling this block function and rebases any unexpected block error to the
  original input offset.

### `decode_64_bytes_avx512`

Location: `src/simd/x86/decode.rs`

Status: admitted std x86/x86_64 AVX-512 VBMI strict decode block for Standard
and URL-safe alphabet families. It is reachable through strict decode dispatch
for full 64-byte encoded blocks after whole-input scalar validation.

Purpose:

- Provide the fixed-block AVX-512 VBMI decode primitive for the admitted strict
  decode boundary without changing scalar public error behavior.
- Exercise AVX-512 6-bit-value packing for a 64-byte encoded block that
  decodes to at most 48 bytes.
- Verify VBMI lane compaction, error agreement, padding behavior, canonical
  trailing-bit rejection, and rejected-input output retention before any later
  dispatch admission.

Preconditions:

- Caller must prove AVX-512 F, BW, VL, and VBMI are available on the current
  CPU.
- Input is exactly 64 encoded bytes.
- Output is exactly 48 bytes.
- The vectorized packing path is used only for Standard-family alphabets
  (`A-Z`, `a-z`, `0-9`, and either `+/` or `-_`). Other alphabets use the
  staged scalar fallback inside the prototype.

Unsafe operation:

- `_mm512_loadu_si512` loads sixty-four 6-bit values from a local staging
  array.
- `_mm512_maddubs_epi16` and `_mm512_madd_epi16` pack groups of four 6-bit
  values into 24-bit decoded quanta within each 128-bit lane.
- `_mm512_shuffle_epi8` compacts the packed quanta into byte order within
  each lane.
- `_mm512_permutexvar_epi8` uses VBMI byte permutation to compact the four
  lane-local 12-byte decoded chunks into a contiguous 48-byte prefix.
- `_mm512_storeu_si512` stores the packed prototype output into a local
  64-byte staging array.
- `clear_zmm_registers_after_encode_block` clears ZMM state and emits
  `vzeroupper` before return to reduce register retention in the vector decode
  prototype.

Safety argument:

- Scalar strict decode validation runs into a private 48-byte staging buffer
  before any byte is copied to the caller output. If validation fails, the
  caller output is left untouched and the staging buffer is wiped.
- The input and output array types provide fixed readable and writable bounds.
- The SIMD loads and store operate only on local 64-byte arrays and use
  unaligned intrinsics, so no alignment precondition is required.
- The AVX-512/VBMI target-feature contract enables every intrinsic used by the
  prototype.
- VBMI compaction indices are constants in `0..=59`, so the permute reads only
  bytes produced by the lane-local decode shuffle.
- The prototype copies only the validated decoded length to caller output after
  an unconditional release-mode equality check proves the vector-packed prefix
  matches the scalar-validation prefix. It wipes local value, packed, and
  scalar-validation staging buffers before returning.
- The public dispatch wrapper validates the complete input with scalar before
  calling this block function and rebases any unexpected block error to the
  original input offset.

### `decode_32_bytes_avx2`

Location: `src/simd/x86/decode.rs`

Status: admitted std x86/x86_64 AVX2 strict decode block for Standard and
URL-safe alphabet families. It is reachable through strict decode dispatch for
full 32-byte encoded blocks after whole-input scalar validation.

Purpose:

- Provide the fixed-block AVX2 decode primitive for the admitted strict decode
  boundary without changing scalar public error behavior.
- Exercise AVX2 6-bit-value packing for a 32-byte encoded block that decodes
  to at most 24 bytes.
- Verify lane-boundary compaction, error agreement, padding behavior,
  canonical trailing-bit rejection, and rejected-input output retention before
  any later dispatch admission.

Preconditions:

- Caller must prove AVX2 is available on the current CPU.
- Input is exactly 32 encoded bytes.
- Output is exactly 24 bytes.
- The vectorized packing path is used only for Standard-family alphabets
  (`A-Z`, `a-z`, `0-9`, and either `+/` or `-_`). Other alphabets use the
  staged scalar fallback inside the prototype.

Unsafe operation:

- `_mm256_loadu_si256` loads thirty-two 6-bit values from a local staging
  array.
- `_mm256_maddubs_epi16` and `_mm256_madd_epi16` pack groups of four 6-bit
  values into 24-bit decoded quanta within each 128-bit lane.
- `_mm256_shuffle_epi8` compacts the packed quanta into byte order within
  each lane.
- `_mm256_storeu_si256` stores the packed prototype output into a local
  32-byte staging array.
- `clear_ymm_registers_after_encode_block` clears lower XMM state and upper
  YMM state before return to reduce register retention in the vector decode
  prototype.

Safety argument:

- Scalar strict decode validation runs into a private 24-byte staging buffer
  before any byte is copied to the caller output. If validation fails, the
  caller output is left untouched and the staging buffer is wiped.
- The input and output array types provide fixed readable and writable bounds.
- The SIMD load and store operate only on local 32-byte arrays and use
  unaligned intrinsics, so no alignment precondition is required.
- The AVX2 target-feature contract enables every intrinsic used by the
  prototype.
- AVX2 byte shuffle is lane-local, so the prototype explicitly compacts the
  second 12-byte decoded lane from offsets `16..28` to `12..24` inside local
  staging before copying to caller output.
- The prototype copies only the validated decoded length to caller output after
  an unconditional release-mode equality check proves the vector-packed prefix
  matches the scalar-validation prefix. It wipes local value, packed, and
  scalar-validation staging buffers before returning.
- The public dispatch wrapper validates the complete input with scalar before
  calling this block function and rebases any unexpected block error to the
  original input offset.

### `clear_xmm_registers_after_encode_block`

Location: `src/simd/x86/cleanup.rs`

Status: private helper for admitted AVX2 and SSSE3/SSE4.1 encode and inactive
x86 prototype tests.

Purpose:

- Clear XMM registers before returning from x86 encode paths that process
  caller bytes in vector registers.

Preconditions:

- Called only after the vector path has stored its output and no later SIMD
  value is needed by the function.

Unsafe operation:

- Inline assembly zeros the XMM register set available to the target (`xmm0`
  through `xmm7` on `x86`, `xmm0` through `xmm15` on `x86_64`) and declares
  those registers as clobbered outputs.

Safety argument:

- The helper does not read or write memory.
- The helper runs at the end of x86 encode block paths.
- Clobbered registers are declared to the compiler with explicit `out("xmmN")`
  operands.
- This is best-effort register-retention reduction, not a guarantee that
  historical register, stack, cache, or microarchitectural copies do not exist.

### `encode_12_bytes_neon`

Location: `src/simd/`

Status: admitted little-endian std AArch64 NEON encode wrapper for Standard
and URL-safe alphabet families. It is reachable through AArch64 encode
dispatch for fixed 12-byte blocks. Big-endian AArch64, 32-bit ARM, unsupported
alphabets, `no_std`, in-place encode, and decode use scalar fallback. Final
tail and padding completion use scalar code.

Purpose:

- Exercise ARM NEON intrinsic plumbing.
- Validate the unsafe boundary on ARM targets.
- Provide the fixed-block vector encode primitive for the admitted AArch64 NEON
  encode backend.
- Keep 32-bit `arm+neon` and custom alphabets on scalar-equivalence scaffold
  paths until their architecture-specific evidence is complete.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- NEON is mandatory on `aarch64`; `arm` builds must enable the `neon` target
  feature.
- Input is exactly 12 bytes.
- Output is exactly 16 bytes.

Unsafe operations:

- On `aarch64` with Standard or URL-safe alphabets, this wrapper calls
  `encode_12_bytes_neon_aarch64_standard_family`.
- On 32-bit `arm+neon`, `vdupq_n_u8` constructs one 128-bit NEON zero vector
  and `vst1q_u8` stores that vector into the output buffer before the scalar
  fallback overwrites the block.
- Custom alphabets use the scalar fallback path.

Safety argument:

- The output type is `&mut [u8; 16]`, so the store has enough initialized,
  writable memory.
- The function is compiled only for `aarch64` or `arm` builds with the `neon`
  target feature.
- The function's safety contract requires runtime NEON availability.
- Runtime dispatch reaches the AArch64 vector path only on little-endian std
  AArch64, where NEON is part of the target contract. Direct tests use the
  same availability precondition.
- Register-retention note: the AArch64 vector path loads caller bytes into NEON
  state and expands `clear_neon_registers_after_vector_block!` directly inside
  the block function before return. This is retention reduction for the
  admitted encode block, not a formal microarchitectural side-channel proof.

### `decode_slice_neon`

Location: `src/simd/neon.rs`

Status: admitted little-endian std AArch64 strict decode dispatch wrapper for
Standard and URL-safe alphabet families. It is reachable only when the `simd`
and `std` features are enabled on little-endian `aarch64`.

Purpose:

- Carve full encoded input blocks into fixed-size array references for the
  NEON target decode block function.
- Preserve scalar public error shape by validating the complete input before
  any NEON block output is copied to caller-visible buffers.
- Fall back to scalar for shorter tails or unsupported surfaces.

Preconditions:

- Runtime dispatch has selected little-endian AArch64 NEON. NEON is mandatory
  for the admitted AArch64 target.
- The input block loop guard proves that each carved block is fully within the
  original input slice.
- The output capacity has been checked against the scalar validated decoded
  length before any block output is copied.

Unsafe operation:

- The wrapper uses `input.as_ptr().add(read).cast::<[u8; 16]>()` and
  dereferences the result to pass a fixed-size block reference to
  `decode_16_bytes_neon`.

Safety argument:

- `read + 16 <= input.len()` is checked before every raw-pointer block carve.
- `read` advances by exactly 16, so the pointer remains within the same input
  allocation and never crosses the slice boundary.
- The wrapper never constructs a mutable alias to input memory.
- Output writes go through a private stack staging buffer first. Bytes are
  copied to caller output only after whole-input scalar validation and block
  equality checks inside `decode_16_bytes_neon`.
- Any unexpected block-level error wipes the local decoded staging buffer and
  rebases the error index to the original input. Tail fallback errors are also
  rebased to the original input offset.
- Unsupported alphabets, short inputs, tails, wrapped decode, legacy decode,
  in-place decode, CT secret decode, `no_std`, and 32-bit ARM stay scalar.
  Wasm decode is admitted only through its separate narrow `simd128` profile.

### `decode_16_bytes_neon`

Location: `src/simd/neon.rs`

Status: admitted little-endian std AArch64 NEON strict decode block for Standard and
URL-safe alphabet families. It is reachable through strict decode dispatch for
full 16-byte encoded blocks after whole-input scalar validation.

Purpose:

- Provide the fixed-block AArch64 NEON decode primitive for the admitted strict
  decode boundary without changing scalar public error behavior.
- Exercise NEON 6-bit-value packing for a 16-byte encoded block that produces
  at most 12 decoded bytes.
- Preserve scalar validation, padding, canonicality, and error behavior as the
  source of truth.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- Input is exactly 16 encoded bytes.
- Output is exactly 12 bytes.
- The function is reached only through the little-endian std AArch64 strict
  decode wrapper or direct tests with the same availability precondition.

Unsafe operation:

- `vld1q_u8` loads a local 16-byte sextet-value staging array.
- NEON shifts, masks, and OR operations pack four 4-symbol quads into decoded
  bytes in 32-bit lanes.
- `vqtbl1q_u8` compacts the first three bytes of each lane into a contiguous
  12-byte prefix using a fixed mask.
- `vst1q_u8` stores into a local 16-byte packed buffer.
- `clear_neon_registers_after_vector_block!` clears `v0` through `v31` inside
  the prototype before return.

Safety argument:

- Scalar decode runs first and returns any malformed-input error before the
  prototype copies bytes to caller-visible output.
- The input and output array types provide fixed readable and writable bounds.
- The vector store writes only to a local 16-byte packed buffer; the caller
  output receives at most the scalar-validated `written` prefix, and only after
  an unconditional release-mode equality check proves the vector-packed prefix
  matches the scalar-validation prefix.
- The compaction mask contains only valid source indices or zero lanes.
- Staging, packed, and scalar-output buffers are wiped before successful return
  or along the error path.
- The NEON target-feature contract enables the required instructions.
- The public dispatch wrapper validates the complete input with scalar before
  calling this block function and rebases any unexpected block error to the
  original input offset.

### `encode_12_bytes_neon_aarch64_standard_family`

Location: `src/simd/neon.rs`

Status: private helper for the admitted AArch64 NEON encode block and its tests.

Purpose:

- Encode one 12-byte fixed block to 16 Base64 bytes using AArch64 NEON for
  Standard and URL-safe alphabets.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- Input is exactly 12 bytes.
- Output is exactly 16 bytes.
- The alphabet must be Standard-family as checked by the caller.

Unsafe operation:

- `vld1q_u8` loads from a local 16-byte staging array that contains the 12-byte
  input plus four zero bytes.
- `vld1q_u8` loads a fixed shuffle mask.
- `vqtbl1q_u8` reshapes staged bytes into four 24-bit lanes without reading
  past the fixed staging array.
- NEON shifts, masks, and OR operations produce sixteen 6-bit indices.
- `encode_standard_family_indices_neon` maps those indices to Standard or
  URL-safe alphabet bytes with NEON comparisons and bit selects.
- `vst1q_u8` stores the 16 encoded bytes into the output buffer.
- `clear_neon_registers_after_vector_block!` clears `v0` through `v31` inside
  the block function before return.
- The local staging array is wiped with the crate cleanup primitive before the
  function returns.

Safety argument:

- The input and output array types provide fixed readable and writable bounds.
- The SIMD load reads only from a local 16-byte staging array, so the helper
  does not over-read the 12-byte caller input.
- The staging array is mutable and wiped after the SIMD store and register
  cleanup, reducing stack retention of the copied caller bytes.
- The function is guarded by a NEON target-feature contract.
- The index vector is masked to `0..=63` before alphabet mapping.
- The output length is fixed by the output array type.
- Runtime dispatch reaches this helper only through the admitted AArch64 NEON
  encode wrapper.

### `encode_standard_family_indices_neon`

Location: `src/simd/neon.rs`

Status: private helper for the admitted AArch64 NEON encode block and its tests.

Purpose:

- Map sixteen 6-bit indices to Standard or URL-safe alphabet bytes with NEON
  comparisons and bit selects instead of scalar per-byte table indexing.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- `indices` contains only byte values in `0..=63`.
- The alphabet must be Standard-family as checked by the caller.

Unsafe operation:

- NEON byte comparisons, arithmetic, and bit-select operations compute the
  ASCII output byte for each index.

Safety argument:

- The helper does not dereference raw pointers or access memory.
- The target-feature contract enables the required NEON instructions.
- The caller constructs `indices` with masks that constrain every byte to a
  six-bit Base64 value.
- The helper is private to the Standard-family NEON encode path.

### `clear_neon_registers_after_vector_block!`

Location: `src/simd/neon.rs`

Status: private macro for the admitted AArch64 NEON encode and strict decode
blocks and their tests.

Purpose:

- Clear AArch64 NEON registers used by the vector block before returning from
  paths that process caller bytes in vector registers.

Preconditions:

- Called only after the vector block has stored its local output and no later
  NEON value is needed by the function.
- Expanded directly inside the vector block function. It must not be moved to a
  separate function because an AArch64 helper can save and restore callee-saved
  `v8` through `v15`, undoing register clearing in the helper frame.

Unsafe operation:

- Inline assembly zeros `v0` through `v31` and declares those registers as
  clobbered outputs.

Safety argument:

- The macro does not read or write memory.
- The macro expands at the end of the NEON vector block path.
- Clobbered registers are declared to the compiler with explicit `out("vN")`
  operands.
- This is best-effort register-retention reduction for SIMD evidence, not a
  guarantee that historical register, stack, cache, or microarchitectural
  copies do not exist.
- This macro clears all AArch64 vector registers for the reviewed encode
  sequence. It is not an admission claim for arbitrary future NEON code.

### `encode_12_bytes_wasm_simd128`

Location: `src/simd/wasm.rs`

Status: private helper for the admitted narrow wasm `simd128` runtime profile.
It is reachable through runtime backend selection only for `wasm32` binaries
compiled with `target-feature=+simd128`, `simd`, and
`allow-wasm32-best-effort-wipe`.

Purpose:

- Select the real wasm `simd128` fixed-block encode helper for Standard-family
  alphabets.
- Keep custom alphabets on scalar fallback logic because portable wasm SIMD
  does not provide a direct 64-byte alphabet lookup instruction.

Preconditions:

- Caller must prove `simd128` is available for the current wasm runtime.
- Input and output lengths are fixed by `[u8; 12]` and `[u8; 16]` arrays.

Unsafe operation:

- Calls the target-feature-gated wasm `simd128` helper.

Safety argument:

- Fixed array types enforce the required block sizes.
- The target-feature contract is explicit on the function.
- Public dispatch reaches the helper only after the feature-gated wasm backend
  reports `simd128` availability and the caller routes through an admitted
  Standard-family encode surface.
- Public dispatch stages this helper's output, compares it against scalar
  output for the same 12-byte block, and copies to caller output only after the
  scalar-verification check succeeds.

Limitations:

- Wasm engines include a runtime/JIT optimization layer outside Rust's compiler
  boundary. This admission is backed by Node/V8, Wasmtime, and
  Chromium-family browser, Firefox/SpiderMonkey, and Safari/WebKit runtime smoke evidence for correctness and dispatch
  reporting, but it does not claim runtime timing, register-retention, or JIT
  zeroization guarantees.

### `encode_12_bytes_wasm_standard_family`

Location: `src/simd/wasm.rs`

Status: private helper for the admitted narrow wasm `simd128` runtime profile.

Purpose:

- Encode one 12-byte block into 16 Base64 bytes with wasm `simd128` byte
  shuffling, vector shifts/masks, and Standard-family alphabet mapping.
- Wipe the staged stack copy before returning.

Preconditions:

- Caller must prove `simd128` is available for the current wasm runtime.
- Input and output lengths are fixed by array types.
- The alphabet must be Standard-family as checked by the caller.

Unsafe operation:

- `v128_load` loads the staged 16-byte array.
- `u8x16_shuffle`, `u32x4_shr`, `u32x4_shl`, masks, ORs, and byte-select
  operations compute the fixed-block output.
- `v128_store` writes exactly 16 output bytes.

Safety argument:

- The staged array is exactly 16 bytes and backs the 128-bit load.
- The output array is exactly 16 bytes and backs the 128-bit store.
- Shuffle lanes that refer to the second input vector read from a zero vector.
- The shifts and masks constrain every encoded index byte to `0..=63`.
- The target-feature contract enables the required wasm SIMD instructions.
- Public dispatch stages this helper's output and compares it against scalar
  output before copying bytes to caller output.

Limitations:

- This helper does not provide a wasm runtime/JIT timing or register-retention
  guarantee. wasm32 cleanup remains governed by the separate fail-closed
  best-effort wipe policy and `allow-wasm32-best-effort-wipe` opt-in.

### `encode_standard_family_indices_wasm`

Location: `src/simd/wasm.rs`

Status: private helper for the admitted narrow wasm `simd128` runtime profile.

Purpose:

- Map sixteen 6-bit indices to Standard or URL-safe alphabet bytes with wasm
  SIMD comparisons and `v128_bitselect` operations.

Preconditions:

- Caller must prove `simd128` is available for the current wasm runtime.
- `indices` contains only byte values in `0..=63`.
- The alphabet must be Standard-family as checked by the caller.

Unsafe operation:

- wasm SIMD byte comparisons, arithmetic, and bit-select operations compute the
  ASCII output byte for each index.

Safety argument:

- The helper does not dereference raw pointers or access memory.
- The target-feature contract enables the required wasm SIMD instructions.
- The caller constructs `indices` with masks that constrain every byte to a
  six-bit Base64 value.
- The helper is private to the admitted wasm encode path.

### `decode_16_bytes_wasm_simd128`

Location: `src/simd/wasm.rs`

Status: private helper for the admitted narrow wasm `simd128` runtime profile.
It is reachable through strict decode dispatch only for Standard and URL-safe
alphabet families after whole-input scalar validation.

Purpose:

- Decode one 16-byte Base64 block into 12 bytes with wasm `simd128` shifts,
  masks, byte shuffles, and stack staging.
- Preserve scalar strict-decode behavior by validating the entire input through
  the scalar decoder before any wasm block writes reach caller output.
- Wipe the staged scalar and decoded stack buffers before returning.

Preconditions:

- Caller must prove `simd128` is available for the current wasm runtime.
- Input and output lengths are fixed by `[u8; 16]` and `[u8; 12]` arrays.
- Whole-input scalar validation has already accepted the full encoded input.
- The alphabet must be Standard-family as checked by the dispatch wrapper.

Unsafe operation:

- `v128_load` loads the staged 16-byte decoded-value array.
- wasm SIMD shifts, masks, ORs, and `u8x16_shuffle` pack the decoded bytes.
- `v128_store` writes exactly 16 bytes into a local scratch array; only the
  first 12 decoded bytes are copied after scalar-equivalence verification.

Safety argument:

- Fixed array types enforce the required block sizes for every load and store.
- Whole-input scalar validation prevents malformed input from reaching caller
  output through the wasm block path.
- The helper compares the wasm block output against the scalar block output
  before returning success.
- Local staging buffers are wiped before the helper returns.
- The target-feature contract enables the required wasm SIMD instructions.

Limitations:

- This helper does not provide a wasm runtime/JIT timing or register-retention
  guarantee. wasm32 cleanup remains governed by the separate fail-closed
  best-effort wipe policy and `allow-wasm32-best-effort-wipe` opt-in.

## Admission Rule

Unsafe SIMD can become an active backend only after scalar differential tests,
fuzz evidence, architecture-specific build evidence, benchmark evidence, and
review of this inventory all pass for that release.
Future SIMD dispatch work must also complete
[SIMD_ACTIVATION_CHECKLIST.md](SIMD_ACTIVATION_CHECKLIST.md).

The admission bar applies equally to AVX2, AVX-512, SSSE3/SSE4.1, NEON, wasm
`simd128`, and any other future vector backend.
For custom alphabets, wrapped, legacy whitespace, in-place, and other
non-standard surfaces, the current scalar/fallback posture is also pinned in
[SIMD_NON_STANDARD_SURFACE_REVIEW.md](SIMD_NON_STANDARD_SURFACE_REVIEW.md).

Any admitted SIMD path that processes caller data must also document its
register-retention cleanup strategy and include the matching explicit register
cleanup implementation, generated-assembly evidence, and tests in the admission
evidence. This is a hard release blocker before dispatch, not an optional
follow-up. Current x86 encode prototypes already load caller bytes into vector
registers and include best-effort register cleanup as test evidence; runtime
dispatch remains blocked until the full admission evidence is complete.
