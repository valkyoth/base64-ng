# Ordinary In-Place Bulk Decoding

Commit 19 preserves the existing public API and transactional error contract.
This is ordinary, input-dependent decoding, not secret storage or a constant-time
operation. Secret staged decoding and reverse in-place encoding are unchanged.

## Validation And Overlap

`Base64::decode_in_place` checks the declared prefix and completes shared ordinary
preflight before taking any mutable output slice. A private capability retains
the same exclusive buffer, settings and checked length. It cannot escape, accept
replacement input, or be constructed by callers. No recoverable error occurs
after compaction starts.

Eligible strict Standard/URL-safe profiles reuse existing backend selection and
health admission. Small inputs, custom alphabets, relaxed policies, unsupported
hosts and builds without acceleration retain scalar compaction. No new ISA or
automatic AVX-512 admission is introduced. Eligible historical `Engine` routes
use the same compactor; canonical rejection leaves the source untouched so the
historical validator can recover its original error. Legacy/wrapped framing and
clear-tail wrappers retain their separate existing contracts.

The final quantum remains reserved for padding and canonical trailing bits.
For the interior, `read` is a multiple of four and `write = read / 4 * 3`.
Each chunk has at most 1024 encoded bytes, all copied to stack scratch **before**
the mutable destination slice is formed. For chunk size `n`, the store covers
exactly `3*n/4` bytes; its end cannot exceed `read+n`. It cannot overwrite the
next unread chunk or the reserved final quantum. The final at-most-four input
bytes are likewise copied before their at-most-three output bytes are written.
No simultaneous overlapping Rust references, raw pointers or new unsafe code
are needed. Existing kernels still receive independent input/output allocations.

## Recovery And Storage

Health is checked again for each chunk. A missing backend uses scalar output.
Kernel rejection quarantines the backend and repairs the whole current chunk
from its preserved source, including any partial stores. Earlier completed
chunks remain correct. With `checked-backend`, independent reference validation
finishes before the first store and each vector output chunk is compared against
the scalar result before acceptance. A mismatch also quarantines and repairs.

The compactor owns 1024 scratch bytes independent of input length. The reused
checked writer adds two 768-byte arrays; these array bounds exclude ordinary
call frames and compiler spills. No heap allocation is introduced. Used scratch
is wiped on normal completion, but caller output and residual tail remain
ordinary non-wiping storage. This does not promise panic/abort secret cleanup.
Callers must still enforce protocol input limits and use secret/CT APIs for
sensitive data.

## Verification

`sh scripts/check-2.1-in-place-bulk.sh` runs active/MSRV feature matrices,
independent-oracle production integration tests, exact malformed diagnostics,
all-byte mutations, short lengths/tails, vector/chunk boundaries, offsets,
residual sentinels, zero-allocation checks and deterministic fuzz regressions.
Fault tests inject rejection, corrupt checked output and backend unavailability
before and after earlier chunks commit. Native Linux guard pages bound both
ends of complete caller buffers. The Miri case forces the copied-source repair
loop without requiring SIMD emulation. The sanitizer gate includes these unit
tests and the production integration suite.

The `in_place` fuzz target now also checks all four canonical codecs against the
independent `base64` crate, using both raw potentially malformed bytes and freshly
encoded valid payloads. Work is bounded to 8192 source bytes for this additional
oracle. This supplements, rather than replaces, historical in-place fuzz cases.

Local implementation verification on 2026-10-05 passed:

- Rust 1.99.0 and MSRV 1.90.0 feature matrices, focused tests and Clippy.
- Complete release workspace tests and workspace Clippy with warnings denied.
- Native x86 overlap, guard-page, fault-recovery and allocation tests.
- Focused Miri and ASan unit/production integration tests.
- Seeded ASan/libFuzzer `in_place`: 10,000 executions, no crash or oracle mismatch.
- AArch64 QEMU checked production integration and fault-recovery tests.
- Complete active/MSRV WASM compilation and Wasmtime scalar/SIMD gate,
  including plain/checked production integration and fault-recovery tests.
- Rustdoc with warnings denied, public API snapshots and release metadata.
- WASM loader: 26 tests, deterministic/path-independent builds, Wasmtime
  self-tests and packed-package installation smoke.

The scalar WASM artifact remains byte-identical. The SIMD artifact was rebuilt
and its embedded loader digest updated. No dependency or public API changed.
The ARM result is emulated functional coverage, not a fresh native ARM, macOS
or Windows run. Independent review, GitHub CI and release campaigns are pending.

## Measurements

The ignored `in_place_bulk_paired_whole_call_comparison` test alternates the new
implementation and the previous scalar compactor with the same full preflight,
input-reset copies, compiler and process. It checks output after each sample.
These local development measurements are not release hardware admission; final
independent review, native target evidence and long fuzz campaigns remain
release tasks.

Local paired results on AMD Ryzen 9 9950X3D, Linux x86_64, Rust 1.99.0,
release `std,simd`, seven alternating pairs per size (without CPU pinning):

| Decoded bytes | Standard padded speedup | URL-safe padded speedup |
| --- | --- | --- |
| 16 | 0.96x | 0.97x |
| 64 | 0.99x | 1.00x |
| 384 | 8.92x | 8.27x |
| 4096 | 15.15x | 16.05x |
| 65536 | 16.29x | 16.36x |
| 1048576 | 16.20x | 15.83x |

Ratios are medians of paired old/new latencies, not ratios against another
library. The 16-byte case increased by about 4-5 ns (3-4%); this is not a claim
of universal improvement. These numbers do not describe checked builds or
historical `Engine` speedups. The local log is
`target/commit19-in-place-benchmark.log`, SHA-256
`906584b0acd007e3689f81f29d32fda5ccbf980ff4b7430b46238a81d5853668`.

```sh
cargo test --locked --release --features simd --lib \
  in_place_bulk_paired_whole_call_comparison -- --ignored --nocapture --test-threads=1
```
