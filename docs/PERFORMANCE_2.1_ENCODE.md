# 2.1 Canonical Encoding Checkpoint

Commit 13 connects ordinary canonical encoding to existing admitted kernels.
This is a development checkpoint, not final release admission or a constant-time
claim. External retest and CI acceptance remain pending.

## Implementation

`Base64::encode_into` retains checked sizing and capacity rejection before any
write. Exact Standard/URL-safe tables qualify independently of decode padding
or trailing-bit acceptance. Encode padding alone chooses padded versus unpadded
output. Inputs below 192 bytes and custom alphabets use the original table
writer; existing per-backend size, CPU, health and static-deployment restrictions
still apply above that floor. No automatic threshold in the historical Engine
dispatcher changes and no new CPU family is admitted.

The selected shared encoder retains `checked-backend` comparison and recovery.
A rejected write or impossible returned length quarantines the selected backend
and the infallible table writer overwrites the complete exact output span.
Capacity errors leave the complete destination unchanged; successful calls
leave spare destination bytes untouched. Allocating and runtime bounded helpers
already forward through this boundary. Compile-time arrays, in-place and
incremental encoding retain their existing implementations.

Internal formatting batches at most 768 input bytes into a 1024-byte ordinary
stack buffer. Formatter/display and counted-sink calls remain four bytes each
(two or three for an unpadded final tail), preserving error progress and partial
write contracts. Owned String append uses whole batches after reservation and
retains rollback on returned errors and unwinding. Short append uses the old
chunk loop. The public `EncodedChunks` iterator is unchanged. These ordinary
buffers do not become secret storage or gain automatic wiping.

## Compiler And Verification

The active compiler is Rust 1.99.0 (`b940084d7`, LLVM 23.1.1), released on
2026-10-01. MSRV remains 1.90.0. CI's compatibility matrix and README table are
updated. New Clippy empty-value assertions use lengths to keep failure output
redacted. Both npm WASM artifacts and embedded digest pins are rebuilt with
the active compiler; loader APIs and package versions are unchanged.

LLVM 23 changes two existing classifier code shapes. SSSE3 input loading is
promoted to the caller; the checker requires per-alphabet exact-width load/call
witnesses and rejects unexpected classifier memory reads. NEON reduces exact
invalid masks with a 64-bit pair sum and full-width zero test. Its instruction
model proves non-cancellation for all possible half-word masks, follows register
aliases and rejects incorrect returns. Both changes have negative mutation
fixtures. Neither checker is a general machine-code proof; the detailed scope
is in [UNSAFE.md](UNSAFE.md).

The legacy NEON evidence generator also recognizes the reviewed valid-mask
pair-sum/full-width comparison shape, with negative fixtures. It now invokes
the stronger validation-only semantic checker as well. Direct NEON kernel
byte/lane, rejection-before-store and boundary tests passed under active/MSRV
QEMU builds; these supplement the writer's instruction-shape inventory.

Focused active/MSRV tests cover no_std, allocation, SIMD and checked features;
all four encoding profiles; every byte at every position in a multi-block input;
length/capacity/alignment boundaries; independent encode/decode policy;
custom-table fallback; direct execution; faults after prior output commits;
reservation failure; no-allocation formatting; and append rollback after a batch.
WASI tests skip only the intentional unwind case because that target aborts.

The AArch64 gate passes under QEMU with active/MSRV execution and production
plain/checked/all-feature ELF assembly checks. Rust 1.99 Apple Silicon Mach-O
assembly also passes cross-inspection; this is not native macOS execution.
On the shared native BananaPi BPI-F3/X60 board, explicit exact-profile admission,
all six ordinary encode tests in plain and checked builds, and all three checked
encode tests passed using cross-built Rust 1.99 executables. No native Rust
installation was changed on that board.

Workspace all-feature tests and Clippy, public API compatibility checks,
active/MSRV encoding and WASM gates, and the focused scalar Miri test for owned,
bounded, append and formatting forwarding passed. The rebuilt loader passed
26 Node tests, Wasmtime self-tests, deterministic/path-independent builds,
package/install smoke and Chromium/Firefox execution. Remaining local CI
sections passed when run separately, including portability, feature/static
contracts, secret/CT checks, docs and dependency audits. No long fuzz or final
release hardware campaign was run for this checkpoint.

## Measurement Method

The baseline is the unchanged parent `0ef1dcb355848fa184fbb556148487d3393ef905`.
Both baseline and candidate use Rust 1.99.0, release optimization, `std,simd`
and the frozen `perf/public-api` harness. Native RUSTFLAGS contain only
`--cfg base64_ng_perf_evidence`; WASI adds `-C target-feature=+simd128`.
This isolates the encoding changes from the compiler update.

`scripts/measure-2.1-encode-public-api.py` measures seven alternating-order pairs
for each profile/size/operation. It runs trusted local executables only, without
the separate untrusted-repository sandbox. The harness checks oracle-equivalent
output outside timing. Non-owning timed operations must allocate zero times.
Ratios are baseline/candidate latency, not raw kernel throughput.

Native measurements use the development Ryzen 9 9950X3D; WASI runs in Wasmtime
on that same host. The desktop is neither isolated nor frequency-locked.
Historical Engine encoding is a control, not an intended algorithm change.
No claim is made for checked-backend throughput, cold KAT latency, all machines,
native ARM performance, Safari performance, or secret APIs. The npm loader still
encodes through the historical Engine, so its benchmark is not a measurement of
this new canonical route.

## Paired Results

Ranges cover Standard/URL-safe, padded/unpadded; x86 bulk selects the existing
AVX-512 VBMI encoder. Every bulk row below meets the harness's exploratory
improvement criterion. Wasmtime is version 46.0.1; guest timers exclude process
startup and JIT compilation.

| Host / raw input size | Canonical caller buffer | Owned String | Append |
| --- | --- | --- | --- |
| x86 / 768 bytes | 3.90-4.00x | 3.09-3.23x | 16.09-18.42x |
| x86 / 64 KiB | 16.92-21.21x | 8.93-11.21x | 19.43-20.51x |
| x86 / 1 MiB | 18.96-19.55x | 8.92-9.36x | 18.86-19.44x |
| WASI simd128 / 64 KiB | 2.71-2.79x | not measured | not measured |

These gains compare the parent's canonical table path, not an already accelerated
historical Engine path. Native 32-byte canonical calls remain within 5 percent;
at 192 bytes ratios span 0.95-1.18x. Tiny canonical regression signals add about
0.2-0.5 ns at empty/three-byte sizes. Native 32-byte append shows roughly 6-7 ns
of overhead, with regression signals for the unpadded profiles. WASI 32-byte
canonical ratios are 0.94-0.98x (within 5 percent or inconclusive). Retaining a
scalar algorithm at small sizes does not promise identical instruction layout
or latency. No universal no-regression claim is made.

Historical controls have no statistically flagged regression in this capture;
one noisy native 64 KiB control has an inconclusive 0.80x median paired ratio.
WASI historical controls remain within 5 percent. The host's background workload
and unlocked clocks limit the strength of all timing conclusions.

Local raw evidence under `target/release-evidence/2.1-commit13-encode/`:

- `native-final.json`: 1568 samples, SHA-256
  `65272b15389cfc528db9c6e8b279ef15488d594aa7ee6768d597715d92ee24a9`.
- `wasm-final.json`: 224 samples, SHA-256
  `1d52f8feafac59ad19e659b713804d4feb05bbdca59b8eb02753a17646941dac`.

Executable SHA-256 values (baseline, candidate respectively):

- Native: `171f090158de07401776533dae13f6842769768d8af5a2f73864b894d393773d`,
  `10843f9f64bf613ea5d8c3fe713e89230489fdba47d2703e3d887ddb05056d7b`.
- WASI: `fd07036d1f79818a56b108e6ab76b03eb8650adcda27177718fe94d13c855d5f`,
  `5359bfbfcb9530426880fa58c28b8a237faf3c434be871fe0adb8670cb61bb7f`.

After building both trusted harness revisions with the identical flags above:

```sh
python3 scripts/measure-2.1-encode-public-api.py BASELINE CANDIDATE > native.json
python3 scripts/measure-2.1-encode-public-api.py BASELINE.wasm CANDIDATE.wasm \
  --wasmtime "$(command -v wasmtime)" > wasi.json
```

The full local `scripts/checks.sh` attempt stops at the pre-existing sandbox
integration test because this desktop's user cgroup lacks `memory.max` even
outside Codex's sandbox. This requirement has not been weakened or skipped in
CI. Results from the remaining individual gates must not be described as a
successful complete local invocation of that script.
