# Progressive Decode Decision

Commit 21 decision: **do not add a public progressive decode API in 2.1.0**.
The ordinary SIMD prototype misses the proposed 1.10x median speedup at two
sizes of at least 4 KiB. This is a decision about the measured design, not a
claim that every possible one-pass decoder must be slower.

Commit 22 will remove the private experiment and retain this decision and its
measurements. No future progressive API is promised. Existing transactional
operations keep their all-or-nothing output contract. Existing `DecoderState`
already supplies bounded incremental progress without another public type.

## Private Experiment

`src/v2/ordinary_decode/progressive_candidate.rs` is compiled only for tests
with std and SIMD on x86_64. It adds no public item, runtime dependency, unsafe
code, production routing, or change to secret/constant-time APIs.

The ordinary successful body is one pass: existing safe SIMD entry points
classify and decode each 1,024-byte encoded block into 768 bytes of stack
scratch, followed by a copy to the caller. There is no whole-input preflight
on that route. CPU/OS availability and backend health are checked before entry;
automatic AVX-512 admission remains excluded. The last 1-1,024 encoded bytes
use the existing transactional tail path. Unsupported/unhealthy acceleration
uses scalar block decoding. Scratch/output are ordinary, input-dependent data,
not secret containers or guaranteed-wiping storage.

The frozen experimental contract is:

- Only strict Standard/URL-safe padded/unpadded settings qualify. Custom and
  relaxed settings return `Unsupported`, without output changes.
- A result contains input bytes consumed and output bytes committed. Success
  means the complete supplied message was consumed, including its final tail.
- Each interior 1,024-byte block commits exactly 768 bytes, only after its
  entire staging operation succeeds. At least the final quantum is reserved.
- A body capacity shortfall returns `OutputFull` before inspecting that block.
  Its minimum output capacity is 768 bytes. The final tail is validated before
  its exact output capacity is checked, so malformed-tail errors take precedence.
- On capacity failure, callers may retry the unconsumed suffix with more output
  capacity. Progress/indexes are relative to the current invocation, not a
  persistent streaming state. There is no buffered or silently consumed input.
- An input error reports the existing reference diagnostic against the complete
  current input, with original indexes. This may require one additional linear
  scan on rejection; it never rewrites the previously committed prefix.
- Bytes after the committed prefix remain unchanged on every returned error.
  Unlike a transactional operation, earlier successful blocks are not rolled
  back on a late padding, trailing-bit, truncation, or alphabet error.
- `checked-backend` independently scalar-decodes every staged vector block and
  compares output before commit. A false acceptance, false rejection, or output
  disagreement quarantines the backend and returns a backend error without
  committing the suspect block. Normal invalid input does not quarantine it.
- Input, capacity and backend errors terminate this invocation. The experiment
  has no resumable decoder lifecycle; only capacity retry of the suffix is
  specified. It must not be substituted for existing transactional wrappers.

Body staging uses a 768-byte array, plus another 768-byte reference array when
checking/falling back. The arrays and final-tail call have bounded stack use;
the decoder allocates no heap storage. No unsafe slices or overlapping input/
output references are created. No fixed total message/output limit is promised.

## Measurements

Captured locally on 2026-10-06, host `valkyoth-heim`, AMD Ryzen 9 9950X3D,
x86_64 Linux, Rust 1.99.0 (`b940084d7`, LLVM 23.1.1). All observations selected
AVX2. This is a warm-buffer development comparison, not native release admission
or a guarantee on other CPUs. Frequency/scheduling were not locked.

Each build compares complete transactional, progressive, and existing
incremental (`update` plus `finish`) operations in the same optimized test
binary. Input is a deterministic byte pattern; output is verified before and
after timing. Each size transfers 4 MiB of decoded bytes per timed batch.
Eleven paired samples rotate operation ordering; three fresh processes repeat
each feature configuration. Startup KAT and backend observations are outside
the timed section. No code is compiled or another test suite run concurrently
with this capture by the measurement driver.

The table gives the range of the three per-process medians of paired
`transactional time / progressive time`; **below 1 means slower**. Sizes are
decoded bytes. Both modes retain strict validation but differ in transactionality.

| Profile | Bytes | Ordinary SIMD | Checked Backend |
| --- | ---: | ---: | ---: |
| Standard padded | 4,096 | 0.741-0.762 | 8.834-8.901 |
| Standard padded | 16,384 | 0.722-0.729 | 8.484-8.507 |
| Standard padded | 65,536 | 0.713-0.722 | 8.387-8.419 |
| Standard padded | 1,048,576 | 0.706-0.712 | 8.401-8.422 |
| Standard unpadded | 4,096 | 0.739-0.758 | 8.554-8.569 |
| Standard unpadded | 16,384 | 0.715-0.730 | 8.160-8.181 |
| Standard unpadded | 65,536 | 0.714-0.718 | 8.044-8.079 |
| Standard unpadded | 1,048,576 | 0.711-0.717 | 8.124-8.161 |
| URL-safe padded | 4,096 | 0.760-0.764 | 8.850-8.896 |
| URL-safe padded | 16,384 | 0.728-0.737 | 8.506-8.519 |
| URL-safe padded | 65,536 | 0.719-0.721 | 8.408-8.431 |
| URL-safe padded | 1,048,576 | 0.709-0.720 | 8.420-8.476 |
| URL-safe unpadded | 4,096 | 0.748-0.757 | 8.512-8.556 |
| URL-safe unpadded | 16,384 | 0.721-0.736 | 8.105-8.219 |
| URL-safe unpadded | 65,536 | 0.712-0.726 | 8.046-8.108 |
| URL-safe unpadded | 1,048,576 | 0.711-0.733 | 8.056-8.116 |

The ordinary path regresses at every measured size/profile: per-block setup,
health checks and staging copies outweigh the avoided vector validation pass
in this implementation. Existing incremental decoding is already close to
transactional bulk throughput at large sizes, without a new public contract.

The large checked-mode gain is real for this experiment, but does not establish
that weakening transactionality is necessary. The current transactional path
also performs a whole-input reference-validation pass through the incremental
reference state machine, in addition to reference decoding during writing. The
prototype omits that pass and instead checks blocks before their commits. The
measurement does not isolate those costs from each other. Improving checked
reference-validation cost while preserving transactionality is a separate
possible optimization, not a promised or admitted change in this commit.

Given the ordinary-path regression, the existing incremental alternative, and
the added error/capacity contract, this release omits the progressive API rather
than exposing a checked-only performance exception.

Raw logs and all paired values are retained in
[`evidence/progressive-2.1/summary.json`](evidence/progressive-2.1/summary.json).
The manifest hashes the measured Rust sources, manifests, toolchain pin,
measurement script and logs. `base_commit` identifies the parent; the source
hashes identify the uncommitted prototype measured on top of it. It is not a
signed release evidence bundle. Profile IDs 0-3 follow the table order.

```sh
python3 scripts/measure-2.1-progressive.py --output target/progressive-new-capture
sh scripts/check-2.1-progressive.sh
```

The measurement command is explicit, bounded and never runs in routine CI.
The correctness gate belongs to the already-deferred development suite.

## Verification

The active/MSRV focused gate passed plain/checked prototype tests, Clippy with
warnings denied, core-only compilation and production LLVM exclusion checks.
Tests cover independent-oracle round trips, all byte values in representative
first/interior/final vector lanes, all terminal sextets, padding, truncation,
alignment offsets, every capacity around multiple blocks, suffix retries,
unchanged sentinels, unavailable backends and injected suspect partial stores.
Transactional API rejection still leaves the whole destination unchanged.
The native lane tests execute existing AVX2 kernels; they do not add ISA admission.

Skeleton mutation tests reject loss of test gating or unauthorized oracle
imports. Measurement parser tests reject missing/duplicate pairs, wrong round
counts, nonpositive timings and scalar-backend captures. The companion cache
follow-up has shell-level regression tests for both target/intermediate paths
and failure propagation; it does not delete old caches.

External pentest and independent CI review of this checkpoint remain pending.
The full release workspace tests, workspace Clippy, unchanged public API
snapshots and a focused Miri unavailable/suspect-block test also passed locally.
The companion gate passed on both toolchains using the isolated directories.
The development package file-count ceiling temporarily includes the three
test-only prototype files; Commit 22 must remove that allowance along with
the experiment before publication is enabled.
No long fuzz, full QEMU, native ARM/Windows, or release evidence campaign was
run or claimed for this private no-go experiment.
