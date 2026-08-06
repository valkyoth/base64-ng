# Unsafe Code Inventory

`base64-ng` keeps scalar encode/decode in safe Rust. The crate root uses
`#![deny(unsafe_code)]`, and reviewed `allow(unsafe_code)` exceptions are
limited to volatile wipe helpers in `src/cleanup.rs`, the constant-time
comparison accumulator barrier and constant-time error gate barrier in
`src/ct/`, and the SIMD boundary in `src/simd/`.

Commit 50 adds no runtime unsafe code. Kani proves portable SIMD arithmetic,
validity masks, wrapper cursor bounds, and initialized-before-visible commit
rules, but does not claim to prove architecture intrinsics, inline assembly,
register cleanup, or runtime feature detection. Those remain connected to the
portable model by direct differential tests, generated assembly, Miri,
sanitizers, and platform evidence. See
[`2.0_FORMAL_VERIFICATION.md`](2.0_FORMAL_VERIFICATION.md).

Commit 51 adds no shipped unsafe code. Its `v2_assurance` target contains a
fuzz-only `unsafe impl ProtectedMemoryProvider` that owns one stable allocation
and asserts the extension contract at every hook. A contract-violating panic
provider is confined to a subprocess regression. Neither implementation is
compiled into a published crate.

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

The integration test `tests/v2_formatting_alloc.rs` contains a test-only global
allocator wrapper. Its `unsafe impl GlobalAlloc` delegates every pointer and
layout unchanged to `std::alloc::System`; atomics only count calls while the
test enables observation. It is not linked into the library artifact.
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
  crate's control. Ordinary public-data builds require no cleanup opt-in;
  `secrets` builds on `wasm32` fail closed unless
  `allow-wasm32-best-effort-wipe` is explicitly enabled. Secret builds on
  unsupported native architectures likewise require
  `allow-compiler-fence-only-wipe` after platform review.
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
  boundary. Ordinary codecs remain portable; `secrets` builds fail closed
  unless `allow-wasm32-best-effort-wipe` is explicitly enabled.
- Unsupported native architectures currently use only the final compiler fence.
  Secret builds fail closed unless `allow-compiler-fence-only-wipe` is
  explicitly enabled after reviewing this weaker cleanup posture and applying
  platform memory controls.
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
- Serve the Commit 14 staged secret decoder without duplicating another unsafe
  optimizer-boundary implementation outside `src/ct/`.

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
- Bound Commit 14 staged secret processing before its public validity and
  success-only plaintext-release branch; Commit 19 owns its timing boundary.

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

### `encode_48_bytes_avx512`, `encode_48_bytes_avx512_inner`, and `encode_full_blocks_avx512`

Location: `src/simd/x86/mod.rs`

Status: admitted x86/x86_64 AVX-512 VBMI encode implementation for Standard and
URL-safe alphabet families. The fixed-array wrapper is test-only. Production
runtime and static-token dispatch enter the private full-block loop. In-place
encode may enter only through stack staging. Unsupported alphabets and tails
use scalar fallback.

Purpose:

- Encode every complete 48-byte input block directly into 64 output bytes.
- Keep exact-width loading, VBMI alphabet mapping, target-feature entry, and
  register cleanup inside one reviewed boundary.

Preconditions:

- Caller must prove the full AVX-512 Base64 candidate bundle is available on
  the current CPU: `avx512f`, `avx512bw`, `avx512vl`, and `avx512vbmi`.
- Input is exactly 48 bytes.
- Output is exactly 64 bytes.

Unsafe operation:

- `_mm512_maskz_loadu_epi8` activates exactly 48 byte lanes and therefore reads
  exactly the fixed input block without a 64-byte over-read.
- `_mm512_loadu_si512` loads a constant expansion index and the 64-byte alphabet
  table.
- `_mm512_permutexvar_epi8` expands sixteen packed three-byte groups into
  sixteen 32-bit lanes without an intermediate stack copy.
- AVX-512 shifts, masks, and OR operations produce sixty-four 6-bit indices.
- `_mm512_permutexvar_epi8` uses the VBMI byte-permute instruction to map those
  indices through the loaded alphabet table.
- `_mm512_storeu_si512` stores the 64 encoded bytes into the output buffer.
- `encode_full_blocks_avx512` advances raw pointers only after slice preflight
  and invokes the inner block for every complete 48-byte input block.
- `clear_zmm_registers_after_encode_block` clears ZMM state and uses
  `vzeroupper` once after the complete production block loop, or once before
  the test-only fixed-array wrapper returns.

Safety argument:

- The input and output array types provide fixed readable and writable bounds.
- The masked load enables exactly lanes `0..48`, so inactive lanes cannot read
  beyond the caller's fixed 48-byte input block.
- Every expansion index is in `0..=47`, and every alphabet index is masked to
  `0..=63` before its table lookup.
- The load and store intrinsics are unaligned variants, so no stronger
  alignment is required.
- The function is guarded by the full AVX-512 Base64 target-feature contract.
- The output length is fixed by the output array type.
- Runtime dispatch reaches this block only after `std` runtime CPU probing
  proves the full AVX-512 VBMI feature bundle. Static `no_std` execution requires
  complete compile-time features or an unsafe deployment attestation, then a
  passing direct KAT and valid thread-bound token.
- Register-retention note: the encoder loads caller bytes into ZMM state. It
  calls `clear_zmm_registers_after_encode_block` after the full block loop and
  before return. This is retention reduction for the admitted encode call, not a formal
  microarchitectural side-channel proof.

### `clear_zmm_registers_after_encode_block`

Location: `src/simd/x86/cleanup.rs`

Status: private helper for admitted AVX-512 VBMI encode/decode calls and tests.

Purpose:

- Clear ZMM state before returning from an AVX-512 call that processes caller
  bytes in vector registers.

Preconditions:

- Called only after the final block has stored its output and no later AVX-512
  value is needed by the function.

Unsafe operation:

- Inline assembly zeros the ZMM register set available to the target (`zmm0`
  through `zmm7` on `x86`, `zmm0` through `zmm31` on `x86_64`) and declares
  those registers as clobbered outputs.
- Inline assembly emits `vzeroupper` to clear upper vector state before
  returning to scalar/SSE code.

Safety argument:

- The helper does not read or write memory.
- The helper runs at the end of the AVX-512 call path.
- Clobbered registers are declared to the compiler with explicit `out("zmmN")`
  operands.
- This is best-effort register-retention reduction for encode evidence, not a
  guarantee that historical register, stack, cache, or microarchitectural
  copies do not exist.

### `encode_24_bytes_avx2`, `encode_24_bytes_avx2_inner`, and `encode_full_blocks_avx2`

Location: `src/simd/x86/mod.rs`

Status: admitted x86/x86_64 AVX2 encode implementation for Standard and
URL-safe alphabet families. The fixed-array wrapper is test-only. Production
runtime and static-token dispatch enter the private full-block loop.

Purpose:

- Encode every complete 24-byte input block directly into 32 output bytes.
- Keep target-feature entry, bounds reasoning, and AVX transition handling in
  one reviewed boundary.

Preconditions:

- Runtime health admission, complete static target features, or the unsafe
  static-token contract must prove AVX2 is available on the current thread.
- The public wrapper preflights the complete output length. Each full-block
  loop iteration then proves 24 readable and 32 writable bytes.
- The vectorized path is used only for Standard-family alphabets (`A-Z`,
  `a-z`, `0-9`, and either `+/` or `-_`). Other alphabets fall back to the
  scalar loop.

Unsafe operation:

- `_mm_loadu_si128` reads bytes `0..16`; `_mm_loadl_epi64` reads bytes `16..24`.
  Shifts, masks, and lane insertion construct two 12-byte lanes with zeroed
  high dwords without a staging copy or caller-memory over-read.
- `_mm256_shuffle_epi8` reshapes each 128-bit lane into four 24-bit groups.
- AVX2 shifts, masks, and OR operations produce thirty-two 6-bit indices.
- `encode_standard_family_indices_avx2` maps those indices with saturated
  subtraction, a class comparison, and a lane-local 16-entry byte-shuffle
  lookup.
- `_mm256_storeu_si256` stores the 32 encoded bytes into the output buffer.
- The complete block loop calls `clear_ymm_registers_after_encode_block` once
  after its final store. That helper clears lower XMM state and emits
  `vzeroupper`; assembly evidence enforces both sequences.

Safety argument:

- The fixed-array inner signature and full-block loop guards provide exact
  readable and writable bounds for every raw-pointer view.
- The two direct loads total exactly 24 bytes; no 32-byte caller load occurs.
- The load and store intrinsics are unaligned variants, so no stronger
  alignment is required.
- The function is guarded by an AVX2 target-feature contract.
- The output length is fixed by the output array type.
- `std` dispatch is runtime-probed. `no_std` dispatch requires complete static
  target features and the generation-bound `StaticBackendToken`, including a
  passing direct KAT and non-quarantined health state.
- Ordinary encode input is classified as public data. The path does not claim
  secret zeroization, but it performs best-effort register cleanup once at the
  complete block-loop boundary. Secret encode remains in the separate scalar
  secret API.

### `encode_standard_family_indices_avx2`

Location: `src/simd/x86/mod.rs`

Status: private helper for the admitted AVX2 encode block and its tests.

Purpose:

- Map thirty-two 6-bit indices to Standard or URL-safe alphabet bytes with a
  branch-free arithmetic classifier and lane-local byte-shuffle lookup.

Preconditions:

- Caller must prove AVX2 is available on the current CPU.
- `indices` contains only byte values in `0..=63`.
- The alphabet must be Standard-family as checked by the caller with a complete
  comparison of positions `0..62` and an explicit check of the two terminal
  symbols.

Unsafe operation:

- AVX2 saturated subtraction and comparison derive a lookup class; a 16-entry
  byte shuffle selects the ASCII offset for each index.

Safety argument:

- The helper does not dereference raw pointers or access memory.
- The target-feature contract enables the required AVX2 instructions.
- The caller constructs `indices` with masks that constrain every byte to a
  six-bit Base64 value.
- The helper is private to the Standard-family AVX2 encode path.

### `clear_ymm_registers_after_encode_block`

Location: `src/simd/x86/cleanup.rs`

Status: private helper for admitted AVX2 encode and strict-decode paths.

Purpose:

- Clear lower XMM state and upper YMM state before returning from an admitted
  AVX2 encode or strict-decode block loop.

Preconditions:

- Called only after the decoder has staged its output and no later AVX/SSE
  value is needed by the function.

Unsafe operation:

- Calls `clear_xmm_registers_after_encode_block` for lower XMM register state.
- Inline assembly emits `vzeroupper` to clear upper YMM state and avoid
  carrying AVX upper halves back to scalar/SSE code.

Safety argument:

- The helper does not read or write memory.
- The helper runs at the end of an admitted AVX2 encode or decode block path.
- `vzeroupper` is valid under the AVX2 target-feature precondition inherited
  from the caller.
- This is best-effort register-retention reduction, not a
  guarantee that historical register, stack, cache, or microarchitectural
  copies do not exist.

### `encode_12_bytes_ssse3_sse41`, `encode_12_bytes_ssse3_sse41_inner`, and `encode_full_blocks_ssse3_sse41`

Location: `src/simd/x86/mod.rs`

Status: admitted x86/x86_64 SSSE3/SSE4.1 encode implementation for Standard
and URL-safe alphabet families. The fixed-array wrapper is test-only;
production runtime and static-token dispatch enter the private full-block
loop.

Purpose:

- Encode every complete 12-byte input block directly into 16 output bytes.
- Keep target-feature entry and exact bounds reasoning in one reviewed
  boundary.

Preconditions:

- Runtime health admission, complete static target features, or the unsafe
  static-token contract must prove SSSE3 and SSE4.1 on the current thread.
- The public wrapper preflights the complete output length. Each full-block
  loop iteration then proves 12 readable and 16 writable bytes.
- The vectorized path is used only for Standard-family alphabets (`A-Z`,
  `a-z`, `0-9`, and either `+/` or `-_`). Other alphabets fall back to the
  scalar loop.

Unsafe operation:

- `_mm_loadl_epi64` reads bytes `0..8`; `read_unaligned::<i32>` reads bytes
  `8..12`; shifts and OR construct the 12-byte lane with a zero high dword.
- `_mm_shuffle_epi8` reshapes the direct input into four 24-bit lanes.
- SSE2 shifts, masks, and OR operations produce sixteen 6-bit indices.
- `encode_standard_family_indices_ssse3_sse41` maps those indices with
  saturated subtraction, a class comparison, and a 16-entry byte-shuffle
  lookup.
- `_mm_storeu_si128` stores the 16 encoded bytes into the output buffer.
- The complete block loop calls `clear_xmm_registers_after_encode_block` once
  after its final store.

Safety argument:

- The fixed-array inner signature and full-block loop guards provide exact
  readable and writable bounds for every raw-pointer view.
- The direct reads total exactly 12 bytes; no 16-byte caller load occurs.
- The load and store intrinsics are unaligned variants, so no stronger
  alignment is required.
- The function is guarded by an SSSE3/SSE4.1 target-feature contract.
- The output length is fixed by the output array type.
- `std` dispatch is runtime-probed. `no_std` dispatch requires complete static
  target features and the generation-bound `StaticBackendToken`, including a
  passing direct KAT and non-quarantined health state.
- Ordinary encode input is public data. The path does not claim secret
  zeroization, but it performs best-effort XMM cleanup once at the complete
  block-loop boundary. Secret encode remains in the separate scalar secret
  API.

### `encode_standard_family_indices_ssse3_sse41`

Location: `src/simd/x86/mod.rs`

Status: private helper for the admitted SSSE3/SSE4.1 encode block and its
tests.

Purpose:

- Map sixteen 6-bit indices to Standard or URL-safe alphabet bytes with a
  branch-free arithmetic classifier and byte-shuffle lookup.

Preconditions:

- Caller must prove SSE4.1 is available on the current CPU.
- `indices` contains only byte values in `0..=63`.
- The alphabet must be Standard-family as checked by the caller with a complete
  comparison of positions `0..62` and an explicit check of the two terminal
  symbols.

Unsafe operation:

- SSE saturated subtraction and comparison derive a lookup class; SSSE3 byte
  shuffle selects the ASCII offset for each index.

Safety argument:

- The helper does not dereference raw pointers or access memory.
- The target-feature contract enables the required SSE4.1 instructions.
- The caller constructs `indices` with masks that constrain every byte to a
  six-bit Base64 value.
- The helper is private to the Standard-family SSSE3/SSE4.1 encode path.

### `decode_slice_ssse3_sse41` and `decode_slice_avx2`

Location: `src/simd/x86/decode.rs`

Status: admitted x86/x86_64 strict decode wrappers for Standard and URL-safe
alphabet families. `std` runtime probing or a thread-bound static `no_std`
token must prove the selected feature bundle.

Purpose:

- Carve full encoded input blocks into fixed-size array references for the
  target-feature decode block functions.
- Preserve scalar public error shape and transactional rejection by validating
  the complete input before any direct SIMD output store.
- Fall back from AVX2 to SSSE3/SSE4.1 and from SSSE3/SSE4.1 to scalar for
  shorter tails or unsupported surfaces.

Preconditions:

- Runtime dispatch has selected only a backend whose CPU features are present.
- The input block loop guard proves that each carved block is fully within the
  original input slice.
- The output capacity has been checked against the scalar validated decoded
  length before any block output is copied.

Unsafe operation:

- Each wrapper carves fixed input and output array references with pointer
  arithmetic after scalar length and capacity preflight.

Safety argument:

- `read + N <= input.len()` is checked before every raw-pointer block carve.
- `read` advances by exactly `N`, so the pointer remains within the same input
  allocation and never crosses the slice boundary.
- Output offsets advance by the exact `16 -> 12` or `32 -> 24` block ratio and
  the validated decoded length proves every fixed store is in bounds.
- The wrapper never aliases immutable input with mutable output. In-place
  callers reach this boundary only after separate stack staging.
- Padding is excluded from SIMD full blocks; final padding and short tails are
  scalar. An impossible post-validation vector classification disagreement
  triggers a complete scalar rewrite of caller output.
- Unsupported alphabets, short inputs, tails, and CT secret decode stay
  scalar. Strict in-place decode may enter this backend only after
  whole-input scalar validation and stack staging. Wrapped and legacy decode
  may enter this strict backend only after scalar line-profile validation,
  line-ending compaction, or legacy-whitespace compaction. Wasm decode is
  admitted only through its separate narrow `simd128` profile.

### `decode_16_bytes_ssse3_sse41`

Location: `src/simd/x86/decode_direct.rs`

Status: admitted x86/x86_64 SSSE3/SSE4.1 direct strict decode kernel for
Standard and URL-safe alphabet families.

Purpose:

- Provide the fixed-block SSSE3/SSE4.1 decode primitive for the admitted strict
  decode boundary without changing scalar public error behavior.
- Classify sixteen caller ASCII bytes, map them directly to 6-bit values, pack
  four quanta, and store exactly twelve decoded bytes.

Preconditions:

- Caller must prove SSSE3 and SSE4.1 are available on the current CPU.
- Input is exactly 16 encoded bytes.
- Output is exactly 12 bytes.
- Whole-input scalar validation has already proved a canonical unpadded full
  block and exact output capacity.

Unsafe operation:

- `_mm_loadu_si128` loads sixteen caller ASCII bytes.
- Signed range comparisons and equality masks classify all alphabet ranges
  and map them directly to 6-bit values.
- `_mm_maddubs_epi16` and `_mm_madd_epi16` pack groups of four 6-bit values
  into 24-bit decoded quanta.
- `_mm_shuffle_epi8` compacts the packed quanta into byte order.
- `_mm_storel_epi64` plus one unaligned four-byte store writes exactly twelve
  bytes without an over-wide caller-output store.

Safety argument:

- Whole-input scalar validation completes before this kernel is entered, so
  malformed input cannot reach any direct output store and exact diagnostics
  remain scalar-defined.
- The input and output array types provide exact readable and writable bounds;
  all loads and stores are explicitly unaligned.
- The SSSE3/SSE4.1 target-feature contract enables every intrinsic used by
  the prototype.
- The validity movemask must contain sixteen accepted lanes before output is
  stored. A disagreement after scalar validation causes scalar fallback.
- `decode_full_blocks_ssse3_sse41` clears XMM state once after the complete
  block loop, not once per block.

### `decode_64_bytes_avx512` and `decode_full_blocks_avx512`

Locations: `src/simd/x86/decode_direct.rs` and `src/simd/x86/decode.rs`

Status: admitted x86/x86_64 AVX-512 VBMI strict decode for Standard and
URL-safe alphabet families. Automatic dispatch uses AVX-512 only at the
retained 16 KiB encoded-input crossover; exact static-token and evidence calls
may use it from one complete 64-byte block.

The safe `decode_slice_avx512` wrapper performs whole-input scalar validation,
output preflight, alphabet checks, tail fallback, and error-index preservation
before and after these unsafe fixed-block helpers.

Purpose:

- Classify sixty-four caller ASCII bytes and map them directly to 6-bit values.
- Pack sixteen Base64 quanta and store exactly forty-eight decoded bytes.
- Batch complete blocks and clear ZMM state once at the call boundary.

Preconditions:

- The caller proves AVX-512 F, BW, VL, and VBMI on the current thread.
- Whole-input scalar validation and output sizing complete before entry.
- Each kernel input is exactly 64 bytes and output is exactly 48 bytes.
- The alphabet belongs to the Standard or URL-safe family.

Unsafe operation:

- `_mm512_loadu_si512` loads exactly sixty-four caller input bytes.
- `map_ascii_to_values_avx512`, `range_mask_avx512`, and `or5_avx512` use
  AVX-512 comparisons and masks to classify every lane before any output store.
- `_mm512_maddubs_epi16`, `_mm512_madd_epi16`, and `_mm512_shuffle_epi8`
  pack each 128-bit lane into twelve decoded bytes.
- `_mm512_permutexvar_epi8` compacts the four lane-local results.
- `_mm512_mask_storeu_epi8` writes only the low forty-eight output bytes.
- `decode_full_blocks_avx512` derives fixed-array references from preflighted
  slice pointers and advances them only by 64-byte/48-byte block widths.
- `clear_zmm_registers_after_encode_block` clears ZMM state once after the
  complete block loop and emits `vzeroupper`.

Safety argument:

- Scalar validation is authoritative for canonicality, padding, detailed
  errors, decoded length, and no-write-on-error behavior. Malformed input never
  reaches the direct block loop.
- Fixed-array references and loop guards prove every unaligned load and masked
  store remains within the preflighted caller slices.
- The kernel requires all sixty-four validity-mask bits before storing. A
  classifier disagreement after scalar validation restarts with scalar decode.
- VBMI compaction indices are constants in `0..=59` and select only bytes
  produced by the lane-local packing sequence.
- The complete target-feature attribute covers every intrinsic. Runtime
  dispatch, static tokens, and exact evidence calls separately prove the same
  feature bundle before entry.
- All vector output is stored before the single call-boundary cleanup.

### `decode_32_bytes_avx2`

Location: `src/simd/x86/decode_direct.rs`

Status: admitted x86/x86_64 AVX2 direct strict decode kernel for Standard and
URL-safe alphabet families.

Purpose:

- Provide the fixed-block AVX2 decode primitive for the admitted strict decode
  boundary without changing scalar public error behavior.
- Classify thirty-two caller ASCII bytes, map them directly to 6-bit values,
  pack eight quanta, and store exactly twenty-four decoded bytes.

Preconditions:

- Caller must prove AVX2 is available on the current CPU.
- Input is exactly 32 encoded bytes.
- Output is exactly 24 bytes.
- Whole-input scalar validation has already proved canonical unpadded full
  blocks and exact output capacity.

Unsafe operation:

- `_mm256_loadu_si256` loads thirty-two caller ASCII bytes.
- AVX2 range/equality masks classify and map every byte directly.
- `_mm256_maddubs_epi16` and `_mm256_madd_epi16` pack groups of four 6-bit
  values into 24-bit decoded quanta within each 128-bit lane.
- `_mm256_shuffle_epi8` compacts the packed quanta into byte order within
  each lane.
- Each 128-bit lane is extracted and stored as exactly twelve bytes, avoiding
  the lane gap and any over-wide caller-output store.

Safety argument:

- Whole-input scalar validation completes before this kernel is entered, so
  malformed input cannot reach direct output and exact diagnostics remain
  scalar-defined.
- The input and output array types provide exact bounds and unaligned
  intrinsics impose no alignment precondition.
- The AVX2 target-feature contract enables every intrinsic used by the
  prototype.
- The validity movemask must accept all thirty-two lanes before either output
  store. A disagreement after scalar validation causes scalar fallback.
- `decode_full_blocks_avx2` clears XMM/YMM state and executes `vzeroupper` once
  after the complete block loop.

### Direct x86 decode mapping helpers

Location: `src/simd/x86/decode_direct.rs`

`map_ascii_to_values_ssse3`, `map_ascii_to_values_avx2`,
`map_ascii_to_values_avx512`, `range_mask_ssse3`, `range_mask_avx2`,
`range_mask_avx512`, `or5_ssse3`, `or5_avx2`, `or5_avx512`, and
`store_12_bytes` are private target-feature helpers used only by the three
fixed-width kernels above. Their range constants are ASCII, alphabet special
bytes come only from a previously recognized Standard/URL-safe table, every
mask operation stays within vector lanes, and `store_12_bytes` requires the
caller's fixed twelve-byte writable range. Exhaustive tests cover every valid
alphabet byte and every invalid byte in every lane before admission.

### Test-only direct x86 decode probes

Location: `src/simd/x86/test_probes.rs`

Status: test-only safe wrappers; excluded from non-test library artifacts.

Purpose:

- Runtime-probe SSSE3/SSE4.1, AVX2, or the complete AVX-512 VBMI bundle.
- Invoke the raw fixed-block classifier before scalar validation so exhaustive
  malformed-lane tests can prove rejection leaves output untouched.
- Clear the corresponding vector register set after each test invocation.

Safety argument:

- Each unsafe kernel call follows a complete runtime feature probe on the same
  thread and receives exact fixed-array input and output bounds.
- Each cleanup call follows the kernel invocation after all output has been
  stored or rejected and no vector result remains needed.
- The wrappers are reachable only from `#[cfg(test)]` code and are not public
  deployment bypasses for the scalar validation boundary.

### `clear_xmm_registers_after_encode_block`

Location: `src/simd/x86/cleanup.rs`

Status: private helper for admitted AVX2 and SSSE3/SSE4.1 encode and strict
decode paths.

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

Status: test-facing wrapper for the admitted little-endian AArch64 NEON direct
encode kernel. Production slice routing uses `encode_full_blocks_neon` and
clears vector state once after the complete block sequence.

Purpose:

- Provide fixed-block test access to the direct AArch64 NEON encode kernel.
- Keep 32-bit `arm+neon` and custom alphabets on scalar-equivalence scaffold
  paths until their architecture-specific evidence is complete.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- NEON is mandatory on `aarch64`; `arm` builds must enable the `neon` target
  feature.
- Input is exactly 12 bytes.
- Output is exactly 16 bytes.

Unsafe operations:

- On little-endian `aarch64` with Standard or URL-safe alphabets, this wrapper
  calls `direct::encode_12_bytes` and then expands the reviewed cleanup macro.
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
- Production dispatch does not use this test wrapper. It processes every full
  block through `encode_full_blocks_neon` and clears vector state once after
  the loop.

### `decode_slice_neon`

Location: `src/simd/neon.rs`

Status: admitted little-endian AArch64 strict decode dispatch wrapper for
Standard and URL-safe alphabet families. It is reachable through runtime
`std` dispatch or a health-gated static `no_std` token.

Purpose:

- Carve full unpadded encoded input blocks into exact fixed-size references for
  the direct NEON decode kernel.
- Preserve scalar public error shape by validating the complete input before
  any direct NEON output is written.
- Fall back to scalar for shorter tails or unsupported surfaces.

Preconditions:

- Runtime dispatch or `StaticBackendToken` has admitted little-endian AArch64
  NEON. NEON is mandatory for the admitted target.
- The input block loop guard proves that each carved block is fully within the
  original input slice.
- The output capacity has been checked against the scalar validated decoded
  length before any block output is copied.

Unsafe operation:

- The full-block loop casts preflighted input/output offsets to exact
  `[u8; 16]` and `[u8; 12]` block references and calls
  `direct::decode_16_bytes`.

Safety argument:

- `read + 16 <= input.len()` is checked before every raw-pointer block carve.
- `read` advances by exactly 16, so the pointer remains within the same input
  allocation and never crosses the slice boundary.
- Output capacity is preflighted and write offsets advance by exactly 12.
- The direct kernel classifies all 16 lanes before its first exact-width output
  store. An unexpected classification disagreement causes a full scalar retry,
  which overwrites any earlier valid-prefix output.
- The final padded quantum is excluded from SIMD and decoded by the scalar tail.
- Vector state is cleared once after the complete block sequence, including an
  unexpected classification failure.
- Unsupported alphabets, short inputs, tails, CT secret decode, `no_std`, and
  32-bit ARM stay scalar. Strict in-place decode may enter this backend only
  after whole-input scalar validation and stack staging. Wrapped and legacy
  decode may enter this strict backend only after scalar line-profile
  validation, line-ending compaction, or legacy-whitespace compaction. Wasm
  decode is admitted only through its separate narrow `simd128` profile.

### `decode_full_blocks_neon`

Location: `src/simd/neon.rs`

Status: private direct-block loop used by the admitted little-endian AArch64
NEON strict decoder after whole-input scalar validation.

Purpose:

- Decode each complete 16-byte, non-padding input block directly into its exact
  12-byte output region.
- Return the input and output offsets for the scalar tail without decoding or
  storing beyond the complete-block prefix.

Preconditions:

- The caller has completed strict scalar validation for the entire input and
  checked the output capacity against the resulting decoded length.
- Runtime dispatch or a health-gated `StaticBackendToken` has admitted NEON on
  little-endian AArch64.
- The caller excludes the final padded quantum from the direct-block prefix.

Unsafe operation:

- Preflighted input and output offsets are cast to exact `[u8; 16]` and
  `[u8; 12]` references before calling `direct::decode_16_bytes`.

Safety argument:

- Every iteration checks that a complete input and output block remains before
  constructing either reference.
- The fixed increments match the exact 16-to-12 Base64 block relationship, so
  neither pointer can leave its originating slice allocation.
- `direct::decode_16_bytes` validates all lanes before either exact-width
  output store. A disagreement with the completed scalar validation aborts the
  direct loop and causes the caller to retry the whole operation through the
  scalar backend.
- The caller clears the used AArch64 vector-register set after the complete
  block sequence and before observing either success or failure.

### `decode_16_bytes_neon`

Location: `src/simd/neon.rs`

Status: test-facing scalar-validation wrapper around the direct little-endian
AArch64 NEON strict decode block. Production uses
`src/simd/neon/direct.rs::decode_16_bytes` after one whole-input validation.

Purpose:

- Preserve the historical test helper while exercising the direct kernel on a
  canonical, unpadded 16-byte block.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- Input is exactly 16 encoded bytes.
- Output is exactly 12 bytes.
- The function is reached only through the little-endian std AArch64 strict
  decode wrapper or direct tests with the same availability precondition.

Unsafe operation:

- Calls the direct kernel after scalar block validation, then clears vector
  state before returning to the test.

Safety argument:

- Scalar validation rejects malformed, padded, or non-canonical test blocks
  before direct output.
- Fixed array types provide exact readable and writable bounds.
- The direct kernel's safety argument is recorded separately below.

### `direct::encode_12_bytes`

Location: `src/simd/neon/direct.rs`

Status: production direct kernel for admitted little-endian AArch64 NEON encode.

Purpose:

- Encode one 12-byte fixed block to 16 Base64 bytes using AArch64 NEON for
  Standard and URL-safe alphabets.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- Input is exactly 12 bytes.
- Output is exactly 16 bytes.
- The alphabet must be Standard-family as checked by the caller.

Unsafe operation:

- `vld1_u8` reads exactly the first eight input bytes and `read_unaligned`
  reads exactly the final four bytes; vector lane construction inserts zeros
  without a 16-byte caller-input over-read.
- `vld1q_u8` loads a fixed shuffle mask.
- `vqtbl1q_u8` reshapes the exact input into four 24-bit lanes.
- NEON shifts, masks, and OR operations produce sixteen 6-bit indices.
- `encode_standard_family_indices` maps those indices to Standard or
  URL-safe alphabet bytes with NEON comparisons and bit selects.
- `vst1q_u8` stores the 16 encoded bytes into the output buffer.

Safety argument:

- The input and output array types provide fixed readable and writable bounds.
- Exact 8+4-byte reads remain within the 12-byte caller input.
- The function is guarded by a NEON target-feature contract.
- The index vector is masked to `0..=63` before alphabet mapping.
- The output length is fixed by the output array type.
- Runtime dispatch reaches this helper only through the admitted AArch64 NEON
  encode wrapper.

### `direct::decode_16_bytes`

Location: `src/simd/neon/direct.rs`

Status: production direct kernel for admitted little-endian AArch64 strict
decode.

Purpose:

- Classify and decode one exact 16-byte unpadded Standard-family block to 12
  bytes without scalar per-block decode, value staging, or scalar comparison.

Preconditions:

- Caller must prove NEON is available on the current CPU.
- Input and output are exact fixed arrays.
- The alphabet must be Standard-family as checked by the caller.

Unsafe operation:

- `vld1q_u8` reads exactly 16 bytes.
- Range/equality masks classify and map ASCII to six-bit values.
- `vminvq_u8` requires every lane to be valid before any output store.
- Shift/mask packing and `vqtbl1q_u8` compact four quanta.
- `vst1_u8` plus `write_unaligned` store exactly 8+4 output bytes.

Safety argument:

- Fixed arrays bound the exact load and stores.
- Output is untouched when any lane is invalid because validity reduction
  precedes both stores.
- The compaction mask contains only valid source indices or zero lanes.
- The target-feature contract enables every NEON instruction.

### `clear_neon_registers_after_vector_block!`

Location: `src/simd/neon.rs`

Status: private macro for admitted AArch64 NEON encode/decode loop boundaries
and direct-kernel tests.

Purpose:

- Clear AArch64 vector registers once after a complete direct block sequence.

Preconditions:

- Called only after the vector block has stored its local output and no later
  NEON value is needed by the function.
- Expanded directly inside the loop owner or test wrapper. It must not be moved to a
  separate function because an AArch64 helper can save and restore callee-saved
  `v8` through `v15`, undoing register clearing in the helper frame.

Unsafe operation:

- Inline assembly zeros `v0` through `v31` and declares those registers as
  clobbered outputs.

Safety argument:

- The macro does not read or write memory.
- Production paths expand the macro once after the full block loop, not once
  per block.
- Clobbered registers are declared to the compiler with explicit `out("vN")`
  operands.
- This is best-effort register-retention reduction for SIMD evidence, not a
  guarantee that historical register, stack, cache, or microarchitectural
  copies do not exist.
- This macro clears all AArch64 vector registers for the reviewed encode
  sequence. It is not an admission claim for arbitrary future NEON code.

### `encode_12_bytes`, `decode_16_bytes`, and `decode_full_blocks` (wasm)

Locations: `src/simd/wasm/direct.rs` and `src/simd/wasm.rs`.

Status: private direct fixed-block helpers for the 2.0 Commit 30 wasm
`simd128` profile.

Purpose and preconditions:

- Encode exactly 12 input bytes into 16 Standard-family Base64 bytes.
- Decode exactly 16 already scalar-validated Base64 bytes into 12 bytes.
- The artifact is compiled with `target-feature=+simd128`; dispatch admits only
  Standard and URL-safe alphabet families.
- Whole-input scalar strict validation, decoded-size calculation, and output
  preflight complete before the direct decode loop.

Unsafe operations:

- Encode uses `v128_load64_zero` plus one four-byte lane load, fixed shuffles,
  shifts, masks, and one exact 16-byte store.
- Decode loads exactly 16 bytes, classifies every lane, reduces the complete
  validity mask, packs four quanta, and stores exactly eight plus four bytes.
- `decode_full_blocks` casts loop positions to fixed-array references only
  under exact block guards and completed output preflight.

Safety argument:

- Fixed arrays and loop guards prove every load and store width without caller
  over-read or over-write.
- Decode performs no output store until `i8x16_bitmask(valid) == 0xffff`.
- The public direct loop is reached only after scalar validation has preserved
  exact error, padding, canonicality, and required-length behavior.
- Padded final quanta and remaining tails are excluded from the direct loop and
  handled by scalar code.
- If the direct classifier unexpectedly disagrees with scalar validation, the
  wrapper discards the partial result by rerunning the complete decode through
  scalar code.

Limitations:

- These are ordinary public-data operations, not secret or constant-time APIs.
- Wasm JIT timing and register retention remain outside Rust's compiler
  boundary. No native register-cleanup guarantee is claimed.

### `base64-ng-wasm-artifact` ABI

Location: `packages/base64-ng-wasm-loader/wasm/src/lib.rs`.

The private artifact crate denies unsafe code by default and locally admits 25
reviewed ABI, static-storage, pointer, and volatile-clear sites. The package
gate fixes that exact count. The unsafe `Sync` implementations depend on one
non-shared wasm instance with synchronous, non-reentrant exports. Raw slice
construction is bounded by fixed 1 MiB input and derived output capacities.
The loader never exports the instance, memory, or pointers and snapshots every
JavaScript input before a call. The linker fixes maximum memory at 128 pages.

`base64_ng_clear_used` clamps caller-provided lengths to the two fixed
capacities. The Rust artifact independently records the high-water mark of
successful output writes and clears the greater of that value and the loader's
reported range, so cleanup coverage does not depend only on duplicated
JavaScript length arithmetic. Volatile writes clear those current ranges after
each operation. `base64_ng_clear` retains complete-capacity volatile clearing
for teardown. Both are best-effort current-memory cleanup only; the JavaScript
package is explicitly ordinary and claims no GC, JIT, register, or
historical-memory erasure. The panic handler traps immediately with the safe
wasm `unreachable` intrinsic rather than consuming a host thread indefinitely.

## 2.0 Web Compatibility Boundary

Commit 16 adds no unsafe code. `web::FORGIVING` is a safe-Rust ordinary parser
with fixed quantum arrays, checked source-length arithmetic, opaque malformed
input failure, and transactional one-shot destination writes. Its browser
evidence does not upgrade it to a constant-time, zeroizing, or secret-capable
surface. Expert `compat` presets reuse the safe ordinary codec core.

## 2.0 Profile And Legacy Compatibility Boundary

Commit 17 adds no unsafe code. Body and alphabet presets are immutable values.
The legacy ASCII-whitespace decoder reuses the safe strict state machine with
an explicit input mode and retains original source indexes before compaction.
It is ordinary, non-wiping, detailed-error behavior and is excluded from all
secret modules.

## 2.0 Secret Storage And Exposure Boundary

Commit 18 adds no new unsafe code. `SecretOutput` and `SecretArray` reuse the
reviewed volatile wipe boundary for complete borrowed or fixed-capacity
storage. `SecretVec` additionally reuses `wipe_vec_spare_capacity`, whose raw
pointer safety argument is documented above. Secret owners do not implement
implicit slice coercions; explicit exposure views retain their owner's cleanup
responsibility, while declassification deliberately transfers that
responsibility to ordinary non-wiping storage. Drop cleanup excludes abort,
forgotten values, process death, and historical hardware or allocator copies.

## 2.0 Assurance And Protected-Memory Boundary

Commit 22 adds five reviewed unsafe declaration or implementation sites. The
safe codec and secret algorithms remain free of unsafe code.

### `AttestationEvidence::new`

Location: `src/v2/assurance/context.rs`

This unsafe constructor binds a reviewed target, wipe procedure, barrier
posture, provider identity, and provider generation. Safe callers cannot mint
attested evidence. The caller must establish every claim for the exact running
deployment and must not replay evidence after provider reconstruction.

### `PlatformAttestation`

Location: `src/v2/assurance/context.rs`

This unsafe trait is the deployment evidence source used to mint an
`Attested` token. Implementations must query or otherwise bind real platform
evidence, return only evidence for the exact provider instance, and never
unwind. A target name, Cargo feature, custom cfg, or requested policy is not by
itself hardware attestation.

### `ProtectedMemoryProvider`

Location: `src/v2/assurance/provider.rs`

This unsafe trait owns protected allocation identity, finite reservation,
physical-protection posture, teardown journaling, quarantine, and disposal.
Implementations must provide unique live handles, generation/ABA safety,
complete-range byte views, allocation-free infallible quarantine transfer, and
non-unwinding hooks. Applied, not-applied, and indeterminate outcomes must be
truthful. Indeterminate disposal must destroy every addressable capability and
leave only a non-owning tombstone identity.

Every ownership-sensitive hook requires a `ProviderAccess` value whose private
constructor is available only to the protected typestate implementation. This
prevents external safe code from directly materializing a default-provider
handle and bypassing ordered cleanup.

The protocol is not a persistent-provider API. The base 2.0 package includes
no persistent teardown provider. The default provider journal and recovery
identity are volatile, valid only for one live provider instance, and cannot
be imported or resumed after restart.

### `BestEffortProvider` provider implementation

Location: `src/v2/assurance/default_provider.rs`

The default provider uses preallocated finite slots and heap allocations. It
does not claim OS page locking, dump exclusion, crash persistence, or hardware
wipe attestation. Its quarantine transfer moves an existing allocation into a
reserved slot without allocating, and bounded maintenance either completes
teardown or permanently quarantines the allocation and shuts down admission.

### `ProtectedSecret` conditional `Send`

Location: `src/v2/assurance/protected.rs`

`ProtectedSecret` is never `Sync`. It is `Send` only when the sealed
`ThreadMovableProvider` proof is implemented inside this crate, the provider is
`Sync`, and the exact handle, typestate, and assurance level are `Send`. No
external provider can opt itself into this capability. The implementation does
not move backing bytes; it only permits moving the unique owner when the
provider contract has been reviewed for cross-thread teardown.

## 2.0 Static Backend Attestation Token

### `StaticBackendToken::assume_supported`

Location: `src/simd/static_token.rs`

This unsafe constructor is the sole caller-attested `no_std` SIMD selection
boundary. The caller must prove the complete CPU feature bundle, enabled OS
vector state, compatible ABI behavior, and that the constructing thread cannot
migrate to an incompatible CPU. A false attestation can execute an unsupported
instruction during the mandatory direct known-answer test and terminate the
process. The token is non-forgeable through safe code, neither `Send` nor
`Sync`, and generation-bound. It bypasses runtime probing only; bounds,
canonicality, KAT, quarantine, and reporting remain mandatory.

Generation validation is a lock-free admission snapshot. Quarantine prevents
later admission but cannot synchronously cancel a call that already observed a
healthy generation. With `checked-backend`, static SSSE3/SSE4.1 and AVX2 encode
and strict decode also use bounded scalar comparison before caller-visible
commit; ordinary fast mode retains the documented one-in-flight-call
revocation window.

Safe static selection uses only compile-time `target_feature` evidence. A
target without pointer-width atomics cannot maintain the required health latch
and remains scalar. Architecture kernel methods are exposed through this token
only by their later backend-specific admission commits.

## Commit 32 Non-Admitted RVV Candidate

Location: `src/simd/rvv.rs`

Commit 32 adds an internal evidence-only RVV 1.0 candidate. It is not compiled
by normal published builds and does not enter `EncodeBackend`, `DecodeBackend`,
or `ActiveBackend` dispatch. QEMU and the pre-admission native X60 campaign
compile it through the project-owned cfg.

Unsafe operations:

- four leaf `global_asm!` functions load exact 12/16-byte blocks, execute RVV
  arithmetic and segmented stores, and clear `v0..v15` at VLMAX;
- one leaf reads the architectural `vlenb` CSR for evidence reporting;
- one native-only leaf keeps a known value in `v8` while waiting without a
  syscall for a one-shot timer signal, then stores the interrupted register
  contents after the signal frame returns;
- one native-only signal-handler leaf deliberately clobbers `v8`, while its
  Rust wrapper records armed signal delivery through an `AtomicU32` before
  return;
- test-only unsafe wrappers `signal_context_round_trip` and `signal_clobber`
  expose those two exact leaf ABIs only to the native evidence harness;
- Rust wrappers `encode_block` and `decode_block` call those symbols through
  `extern "C"` after fixed-block bounds checks and an RVV/vector-state gate;
- Linux runtime detection calls `getauxval`, `riscv_hwprobe` through `syscall`,
  and `prctl(PR_RISCV_V_GET_CONTROL)` with the kernel UAPI layouts.

The assembly functions are stackless leaves, make no nested calls, use only
caller-saved integer temporaries plus vector registers, and carry CFI function
boundaries. Scalar validation completes before strict-decode output writes.
The UAPI probe uses one valid writable pair, a zero-sized null CPU set, and
fails closed on syscall errors, unsupported keys, missing `V`, or disabled
vector state. The wrappers repeat the probe at each candidate entry and never
process-cache `PR_RISCV_V_GET_CONTROL`, whose result applies to the calling
thread. QEMU's older-kernel fallback accepts only the startup `AT_HWCAP` `V`
bit.

Generated disassembly, ELF attributes, VLEN 128/256 QEMU execution, and pure
probe-result tests are required by the Commit 32 gates. The signal test is
ignored by QEMU and ordinary suites and invoked by exact name only on native
hardware. Its process-global handler and one-shot timer are installed,
disabled, and restored by the sole test in that harness. The helper uses
aligned atomic words for its bounded wait and confirms both armed delivery and
restoration of the interrupted vector register without expecting vector state
to survive a Linux syscall. These remain pre-admission candidate evidence.
Accepted native correctness, ABI/signal preservation, performance,
register-cleanup review, exact-profile dispatch integration, and an external
pentest remain hard requirements before production admission.

## Commit 33 Non-Admitted SVE Candidate

Location: `src/simd/sve.rs`

Commit 33 adds an internal QEMU-only AArch64 SVE candidate. It is compiled
only through the project-owned `base64_ng_sve_candidate` evidence cfg and does
not enter `EncodeBackend`, `DecodeBackend`, or `ActiveBackend` dispatch.

Unsafe operations:

- four leaf `global_asm!` functions load exact 12/16-byte blocks with
  `ld3b`/`ld4b`, execute predicate-based Standard or URL-safe mapping, store
  exact 16/12-byte blocks with `st4b`/`st3b`, and clear caller-saved
  `z0..z7` plus `p0..p1` before return;
- one stackless leaf reads the architectural vector length with `cntb` for
  evidence reporting;
- Rust wrappers call those symbols through `extern "C"` only after exact block
  bounds and the per-call SVE/vector-length gate are proven;
- Linux/Android runtime detection calls `getauxval(AT_HWCAP)` and
  `prctl(PR_SVE_GET_VL)` through their reviewed C ABI, and test-only evidence
  changes the current thread's vector length with `PR_SVE_SET_VL`.

Every data kernel activates exactly four byte lanes with `ptrue p0.b, vl4`, so
its memory footprint and result are independent of the physical SVE vector
length. The functions are stackless leaves, make no nested calls, use only
base-PCS caller-saved vector and predicate registers, and carry CFI function
boundaries. Scalar whole-input validation completes before strict-decode
output writes; padded final quanta and tails remain scalar. Capability results
are queried for every candidate call rather than cached because SVE vector
length is per-thread and can change through `prctl`.

Generated disassembly, QEMU execution at 128-, 256-, and 512-bit vector
lengths, malformed probe tests, per-thread vector-length changes, and a static
`no_std +sve` build are required by the Commit 33 gates. These are functional
candidate evidence only. Real-hardware correctness on at least two vector
lengths, ABI/signal preservation, performance, register-remanence review, and
an external pentest remain hard requirements before production admission.

## Admission Rule

Unsafe SIMD can become an active backend only after scalar differential tests,
fuzz evidence, architecture-specific build evidence, benchmark evidence, and
review of this inventory all pass for that release.
Future SIMD dispatch work must also complete
[SIMD_ACTIVATION_CHECKLIST.md](SIMD_ACTIVATION_CHECKLIST.md).

The admission bar applies equally to AVX2, AVX-512, SSSE3/SSE4.1, NEON, wasm
`simd128`, and any other future vector backend.
For custom alphabets, in-place extensions, and other non-standard surfaces, the
current scalar/fallback or staged-admission posture is also pinned in
[SIMD_NON_STANDARD_SURFACE_REVIEW.md](SIMD_NON_STANDARD_SURFACE_REVIEW.md).
Wrapped and legacy decode may enter the admitted strict decode backend only
after scalar line-profile validation, line-ending compaction, or
legacy-whitespace compaction; those line and whitespace handling stages remain
scalar.

Any admitted SIMD path that processes caller data must also document its
register-retention cleanup strategy and include the matching explicit register
cleanup implementation, generated-assembly evidence, and tests in the admission
evidence. This is a hard release blocker before dispatch, not an optional
follow-up. Current admitted x86 encode and strict-decode backends load caller
bytes into vector registers and include best-effort register cleanup plus
generated-code evidence. Commit 24 additionally places ordinary accelerated
execution behind direct KAT, generation, and quarantine checks; future
architecture rewrites must renew the complete admission evidence.
