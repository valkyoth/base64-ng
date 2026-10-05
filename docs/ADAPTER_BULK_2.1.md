# Bounded Adapter Bulk Checkpoint

Commit 18 connects the ordinary incremental bulk core to bounded user I/O.
External review and CI passed at `3f459f0`. This is development verification,
not final release admission or a new native hardware performance claim.

## Contracts

- The synchronous decoder writer now batches complete output-fitting quanta.
  It retains the 1024-byte queue, the old accepted-input ceiling (including
  pending input), and bounded 1024-byte scratch. It does not collect a complete
  message or allocate proportional to input.
- A cloned driver validates the candidate prefix. Malformed input replays the
  original quantum loop so exact errors, accepted prefixes, pending state and
  queue contents remain compatible. Backend failures are not treated as syntax
  fallback: quarantine, absorbing failure and checked recovery still apply.
- Synchronous encoding, Tokio readers/writers and Bytes already forwarded
  substantial spans to the incremental core. They retain their capacities and
  cumulative limits. Ring queues now insert with at most two bounded copies,
  while consumed and dropped storage retains its existing cleanup.
- The synchronous decoder reader deliberately retains quantum-sized source
  reads. A padded frame must leave its adjacent payload unread. Tokio
  `new_exact` retains declared frame boundaries; EOF mode still consumes to EOF.
- Finalization handles a pre-existing full-queue edge: a valid tail that cannot
  fit triggers a drain before its original state is consumed. A drain error
  leaves that tail retryable. A wiped clone checks validity first, so invalid
  tails still fail before draining any queued prefix. No queue grows.

Streaming is prefix-committing, not a transaction across a complete message.
Already delivered ordinary bytes cannot be recalled on a malformed suffix,
I/O failure or cancellation. Ordinary output is neither a secret container nor
a constant-time result. Enforce protocol input/output ceilings before accepting
frames. Dropping a pending future is not dropping the adapter: resumable work
retains its bounded state, while adapter drop retains existing cleanup rules.

## Validation Policy

Synchronous `stream::Decoder` and `stream::DecoderReader`, plus Tokio
`DecoderWriter` and `DecoderReader`, provide consuming `with_validation`
builders. `BytesDecoder` provides per-call `update_with_validation`:

```rust
use base64_ng::{DecodeValidation, STANDARD, stream::Decoder};
use std::io::Write;

let mut decoder = Decoder::new(Vec::new(), STANDARD)
    .with_validation(DecodeValidation::ScalarReference);
decoder.write_all(b"Zm9v").unwrap();
assert_eq!(decoder.finish().unwrap(), b"foo");
```

`Auto` is unchanged as the default. `ScalarReference` validates newly accepted
input with the independent scalar state machine but may still use an admitted
writer. Previously accepted pending output is not retroactively revalidated.
Frame bounds, syntax, strict tail checks, backpressure and cumulative limits
are independent of this choice. No progressive or unchecked API is introduced.

## Examples And Checks

```sh
cargo run --example stream_file --features stream -- INPUT OUTPUT
cargo run -p base64-ng-tokio --example stream_transfer
sh scripts/check-2.1-adapter-bulk.sh
```

The file example refuses an existing output path (including symlinks), creates
Unix output with mode `0600` before any bytes are written, caps input at 64 MiB
plus one overflow-detection byte, and explicitly finalizes/syncs output. Use a
trusted destination directory; on Windows its inherited DACL must explicitly
restrict access. Base64 does not provide confidentiality. Any error
can leave a partial output file that must be discarded. The async example uses
a bounded duplex channel, exact frame length, a decoded-output ceiling and
explicit shutdown; it never buffers an unbounded source.

Regression coverage includes a frozen quantum-loop comparison for all four
strict profiles, every malformed position, both policies and partial pending
quanta. It compares complete queue contents, driver state, errors and accepted
input. Fault injection covers validation rejection and rejected writer output.
Adapter tests exercise queue bounds, partial/zero/interrupted writes, retry,
Pending cancellation/resumption, exact frame adjacency, premature EOF,
fragmented input/output, limits, malformed suffixes and existing drop/unwind
cleanup. The finalization regression was reproduced on parent `df2fe8f` before
the fix; encoder and decoder regressions include a failed drain followed by
successful retry, plus invalid-tail rejection without a drain.

The focused gate runs active Rust 1.99.0 and MSRV 1.90.0 in stream-only, SIMD
and checked configurations, plus Clippy, Bytes core-only tests and both examples.
Each file-example regression builds once in a fresh temporary directory
(Unix mode `0700`) with build umask `077`. Both Cargo output and intermediate
build directories are private; compiler-cache wrappers are disabled for this
test. The executable from Cargo's JSON artifact message must resolve inside
the private target. Only that binary runs under umask `000`, while the private
directory remains alive. Success and failure both clean it up. Cargo, rustc
and build scripts never inherit the permissive test umask, and the test never
reuses the normal target cache. Runner regressions enforce these boundaries.
The example execution verifies mode `0600` and encoded content, and rejects
existing destinations, input-as-output and
existing/dangling symlink destinations without modifying their targets.
Other commands may still use old shared build artifacts. Hosts that ran the
earlier Cargo-under-umask-`000` test should discard and rebuild affected caches
before reuse (for its default debug build,
`cargo clean -p base64-ng --profile dev`). Changing the umask alone does not
repair existing artifacts or establish that they were not modified.
Bounded Miri tests cover pending/rejected bulk work and finalization retries.
Seeded ASan/libFuzzer campaigns cover `stream_chunks` and `v2_async` for 10,000
executions each. These short campaigns do not replace release fuzzing.

Local verification completed: the active/MSRV adapter gate, release workspace
all-feature tests, workspace/all-target Clippy, warnings-denied rustdoc, API
snapshots, the three bounded Miri tests and both seeded fuzz campaigns passed.
The complete repository check sequence passed; its cgroup isolation gate needed
host user-bus access outside the coding sandbox and passed all 12 tests there.
The remaining sequence was run separately after that environment-only stop.
Both WASM artifacts rebuilt unchanged, all 26 loader tests passed, and the
existing Wasmtime, portability, dependency-policy and RustSec checks passed.
No fresh native ARM/Mac/Windows run or long fuzz campaign is claimed.

## Measurement Method

The trusted development runner measures real synchronous, Bytes and Tokio
adapters, including construction and finish/shutdown. Short writers constrain
output by the fragment limit; the Tokio sink alternates `Pending` and ready
short writes, waking the caller each time. Buffers are allocated outside timing;
timed operations must allocate zero times and results are checked independently.

Both builds use the same updated `perf/public-api/src/adapters.rs` harness,
Rust 1.99.0, `std,simd,adapters` and `--cfg base64_ng_perf_evidence`. The baseline
is `df2fe8fd61c0c514bfd66d29f7792c1522e0b495`. Seven alternating paired samples
cover four strict profiles, encode/decode, 32 B/768 B/64 KiB/1 MiB payloads and
7/256/4096/1048576-byte fragments, pinned to CPU 2 on a Ryzen 9 9950X3D.
This desktop comparison is not an untrusted-code sandbox or portable guarantee.

```sh
RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo build --locked --release \
  --manifest-path perf/public-api/Cargo.toml --features simd,adapters
taskset -c 2 python3 scripts/measure-2.1-incremental.py BASELINE_BINARY \
  perf/public-api/target/release/base64-ng-public-api-perf \
  --operations sync bytes tokio > paired.json
```

Copy the identical harness into a frozen parent checkout before building it.
Local raw captures are retained under
`target/release-evidence/2.1-commit18-adapters/`. No native ARM, Apple Silicon,
Windows or RISC-V adapter-performance claim is made here.

### Native Results

Paired parent/candidate latency ratios across the four profiles (larger is
better), from `paired-final.json`:

| Decoded payload | Fragment limit | Sync decode ratio |
| --- | --- | --- |
| 32 B | 7 B | 0.98-1.00x |
| 768 B | 4 KiB | 21.74-22.71x |
| 64 KiB | 4 KiB | 57.01-59.67x |
| 1 MiB | 4 KiB | 55.45-58.07x |
| 1 MiB | 7 B | 1.00-1.03x |

Standard padded 1-MiB decode with 4-KiB fragments changed from 52.29 ms to
0.924 ms. These ratios compare the old quantum-at-a-time synchronous decoder
with bounded bulk processing, not another library or one-shot decoding.
Tokio and Bytes already used the bulk core; this commit does not claim similar
gains there. All 384 cases had zero timed allocations on both sides.

The final sweep classified 326 cases within 5 percent, 27 as improvement
signals, 20 inconclusive, 10 noisy and one regression signal. That last case
was Bytes unpadded Standard encode at 64 KiB/7-byte fragments: 0.948x, or
260 us to 274 us. A longer 21-pair, 100-round recheck measured 0.969x
(271 us to 277 us), within 5 percent but still slightly slower. Four Bytes
signals from the earlier capture also did not reproduce as regressions in
the longer recheck. Both captures and `bytes-recheck.json` are retained;
timing/code-layout variation must not be represented as universal no-regression.

Final capture SHA-256:
`565c951d4eb02a4a65d1da76f0b0ab8f029b8d37e1aadd7f551f912a3374c400`.
Baseline executable SHA-256:
`05db5fe55f70ac4b8aa92ae88f94d3d508d10012e0f504d50e315145da056ade`.
Candidate executable SHA-256:
`f1973eb8beafb79562df31505066d646a14e11ea75bf98464c0a43ab14c0819a`.
