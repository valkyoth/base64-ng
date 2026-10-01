# Commit 10 NEON Measurements

This is a bounded development experiment, not release admission or a general
performance guarantee. The first two campaigns measured a test-only candidate;
the integration campaign below measures the production public APIs, including
health selection. Do not treat the earlier candidate numbers as integrated
results. Static-token and secret/CT contracts are unchanged.

## Scope And Method

Measured on 2026-10-01 on AWS AArch64 Linux, four-core ARM Neoverse-V2,
Rust 1.98.1, release mode, `--no-default-features --features std,simd`. Source
is the Commit 10 working tree based on accepted
`d45a48b26507220ef8a0763dadc235ab0f139c20`, with the shared strict scalar tail
validator (not the earlier exploratory reference-tail implementation).

Seven samples rotate three complete operations in the same process: canonical
public Auto, canonical public ScalarReference, and the private NEON candidate.
The candidate includes family selection, vector prefix and scalar tail
validation, preflight/capacity checks, complete output writing and register
cleanup. Small inputs without a complete interior block do not execute NEON.
Inputs and outputs are allocated before timing. An independent RFC oracle
provides all four Standard/URL-safe padded/unpadded fixtures; outputs are checked
before and after each timed sample, with success and length checked every call.
Rounds are 10000 for 0/3/32-byte payloads, 500 for 1024 bytes, 32 for 65536 bytes,
and 4 for 1048576 bytes. The benchmark prints 504 sample records.

## AWS Results

Standard padded median nanoseconds per complete decode; sizes are decoded
payload bytes. Speedup is Auto time divided by candidate time.

| Payload bytes | Public Auto | ScalarReference | NEON candidate | Speedup |
| --- | ---: | ---: | ---: | ---: |
| 0 | 22.95 | 38.05 | 16.32 | 1.41x |
| 3 | 46.29 | 201.94 | 20.58 | 2.25x |
| 32 | 63.08 | 1638.16 | 40.72 | 1.55x |
| 1024 | 841.09 | 49537.67 | 691.81 | 1.22x |
| 65536 | 51819.94 | 3109637.03 | 43003.88 | 1.21x |
| 1048576 | 841824.50 | 49482517.50 | 674575.50 | 1.25x |

Across all four profiles, 64 KiB speedup is 1.205-1.207x and 1 MiB speedup is
1.243-1.248x. These warm-buffer, fixed-pattern results do not establish a
crossover, cold-cache behavior, or invalid-input throughput. The private
candidate has no production backend health-selection
overhead yet; small-input numbers in particular are not promised public API
improvements. `ScalarReference` is the canonical policy, not the historical
Engine decoder. No comparison with other libraries is implied.

The raw final samples and native/QEMU gate logs are retained locally under
`target/release-evidence/2.1-commit10-neon/`; this is not a signed release bundle.

## Apple Silicon Results

The maintainer supplied a passing native correctness/codegen gate for commit
`c3c59d36bcf6df7197d2070ad13ef692515f9740`, Rust 1.98.1 / LLVM 22.1.8,
`aarch64-apple-darwin`, followed by the full benchmark log from the command
below. The chip model and macOS version were not supplied. Commit/compiler
identity comes from the accompanying terminal transcript, not metadata embedded
in the benchmark file; these are operator-reported development results.

All 504 expected records were checked for unique profile/size/sample/mode keys,
expected round counts, positive timings, and a successful test summary. The
correctness gate passed for active/MSRV compilers and checked/non-checked
configurations. Its zero-test guard-page entries on macOS are expected: that
test is Linux-only and was executed separately on AWS, not on this Mac.

Standard padded median nanoseconds per call follow. Here speedup is the median
of seven paired Auto/candidate time ratios, preserving sample pairing rather
than dividing independently selected median times.

| Payload bytes | Public Auto | ScalarReference | NEON candidate | Paired speedup |
| --- | ---: | ---: | ---: | ---: |
| 0 | 24.35 | 45.15 | 32.65 | 0.75x |
| 3 | 20.15 | 156.33 | 16.18 | 1.25x |
| 32 | 36.23 | 1365.86 | 33.50 | 1.08x |
| 1024 | 642.25 | 42635.75 | 416.67 | 1.54x |
| 65536 | 40522.13 | 2737065.09 | 25286.47 | 1.60x |
| 1048576 | 663854.25 | 44016468.75 | 409541.75 | 1.62x |

Across all four profiles, paired speedups are 1.587-1.603x at 64 KiB and
1.599-1.632x at 1 MiB. Empty-input ratios are only 0.746-0.828x: the candidate
is slower, so these results do not justify unconditional routing. Preserve the
existing empty-input fast path during production integration and remeasure the
integrated path, including its health checks. No statistical admission or
universal improvement is claimed.

The supplied file is retained locally as
`target/release-evidence/2.1-commit10-neon/macos-benchmark.log`, SHA-256
`20fb1109e0c80d8e5702dea2f8029b6b97f6161a1033e48e97de6b6f28d90fbe`.

## Native Reproduction

On Apple Silicon and AWS ARM separately, from the same reviewed commit:

```sh
git status --porcelain
git rev-parse HEAD
rustc -Vv
sh scripts/check-2.1-neon-validation.sh
cargo test --locked --release --no-default-features --features std,simd \
    --lib v2::ordinary_decode::neon_candidate::benchmark::same_process_decode_comparison \
    -- --ignored --exact --nocapture --test-threads=1
```

Retain the commit, compiler, machine model, gate output and all benchmark rows.
The gate installs missing pinned/MSRV toolchains and targets and runs both
compiler feature matrices. Linux also runs guard pages. The same assembly
checker accepts ELF and Mach-O syntax and verifies exclusion of the asserting
test candidate, not the production classifier. The expanded integrated gate
also runs public validation-policy, health and injected-fault recovery tests.
Apple Silicon candidate timings are recorded above; integrated correctness
results are recorded below. At an integrated revision, the optional same-process
benchmark compares integrated Auto with reference and private candidate, not
with the earlier public implementation.

On a non-ARM Linux host with QEMU installed, use
`sh scripts/check-2.1-neon-validation.sh --qemu` for functional tests only.
Without that option the gate cross-compiles and checks codegen, explicitly
reporting that execution was not performed. QEMU timings are not hardware
performance evidence.

## AWS Production Integration

On the same Neoverse-V2 host, Rust 1.98.1, two separate release builds of the
unchanged `perf/public-api` harness compared candidate-checkpoint production
source `c3c59d36bcf6df7197d2070ad13ef692515f9740` with the integration working
tree based on `6ea850d`. Both used `--features simd` and
`RUSTFLAGS='--cfg base64_ng_perf_evidence'`; checked-backend was not enabled for
timing. Each operation/profile/size had seven paired samples, alternating which
binary ran first. No correctness builds ran concurrently with this measurement.

The harness independently verifies output and length outside the timer, counts
successful calls inside it, checks zero timed allocations, and reports NEON capability
for both builds. Patterns are deterministic `random`, buffers are warm, and all
four strict profiles are included. Round counts are 100000 for 0/3/32 bytes,
5000 for 1024, 2000 for 3072/4096, 128 for 65536, and 8 for 1048576. Sizes are
decoded payload bytes; ratios are medians of paired old/new elapsed times.
This is a development comparison, not a statistical admission or a comparison
with a competitor, cold-cache workload, or checked-backend performance.

An initial 512-encoded-byte floor regressed canonical 1 KiB calls (0.844-0.848x
ratios) despite large-input gains. The integration therefore uses a conservative
4096-encoded-byte NEON floor, leaving x86's 512-byte floor unchanged. The final
run produced these all-profile ranges:

| Payload bytes | Canonical decode | Historical `decode_slice` | Canonical validate |
| --- | ---: | ---: | ---: |
| 1024 | 1.003-1.008x | 0.999-1.000x | 0.992-1.002x |
| 3072 | 1.046-1.047x | 2.147-2.726x | 1.336-1.344x |
| 4096 | 1.079-1.080x | 2.242-2.847x | 1.402-1.406x |
| 65536 | 1.193-1.194x | 3.821-4.596x | 1.688-1.720x |
| 1048576 | 1.249-1.256x | 4.291-4.851x | 1.778-1.783x |

The 0/3/32-byte paths do not enter vector selection. Their few-nanosecond
differences are not claimed as improvements: canonical ratios range from
0.907-1.116x and historical empty calls from 0.728-0.833x (about 1-2 ns slower).
The early return preserves the small-input algorithm, not identical codegen or
timing. This does not establish a universal crossover on all AArch64 CPUs.

All 1344 timed records are retained locally in
`target/release-evidence/2.1-commit10-neon/integrated-aws-public-api.json`, SHA-256
`529f9403602c8ecea152fd5509e00b7d6008338d1cf94d9e02115efc8affb113`.
The local evidence directory also retains the runner and source hashes; it is
not a signed release bundle. Native AWS and QEMU feature/MSRV gates cover
the integrated kernels, guard pages, health faults and public transactionality.
Apple Silicon paired integration timings and the subsequent external pentest
acceptance are recorded below, separately from this AWS measurement.

For a direct public operation sample, build this same harness at each revision
separately, then alternate its binaries with these arguments (repeat for all
profiles and sizes for a paired comparison):

```sh
RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo build --locked --release \
    --manifest-path perf/public-api/Cargo.toml --features simd
perf/public-api/target/release/base64-ng-public-api-perf \
    canonical decode sp 65536 random 4096 128 warm
perf/public-api/target/release/base64-ng-public-api-perf \
    historical decode sp 65536 random 4096 128 warm
```

## Apple Silicon Integration Correctness

The maintainer supplied a terminal transcript identifying
`450239a0696a9efde63e06448931649e3989e013` on `aarch64-apple-darwin` and
successful completion of both commands:

```sh
sh scripts/check-2.1-neon-validation.sh
cargo test --locked --release --all-features
```

The gate passed its pinned Rust 1.98.1/MSRV 1.90.0 feature matrices, plain and
checked public routing, validation-policy and health tests, injected-fault
recovery, production-IR exclusion of the asserting candidate, assembly and
unsafe/panic policies. The zero-test guard-page invocations are expected on
macOS: those tests are Linux-only and were exercised on AWS separately.

The root package's all-features run passed 254 unit tests, 209 integration tests
and 75 doctests. The one ignored unit test is the opt-in development benchmark.
Repeated sections in the supplied paste describe the same run and are not
counted as independent executions. This is operator-reported correctness
evidence; no new benchmark, compiler identity transcript, chip model or macOS
version was supplied with this rerun. It does not replace paired integrated
performance measurements or external security review.

## Apple Silicon Production Integration

The maintainer supplied `base64-neon-paired-20261001-161958.json` from an Apple
M2 Pro running macOS 27.0.1 (26A434), Rust 1.98.1, LLVM 22.1.8. It compares
`c3c59d36bcf6df7197d2070ad13ef692515f9740` with
`199a125cea9d4f06fa7604c14a3ff0553c5cda53`. The `perf/public-api` harness is
unchanged between these revisions. The supplied runner builds separate release
binaries with `--features simd` and `RUSTFLAGS='--cfg base64_ng_perf_evidence'`,
then alternates execution order for seven paired samples per case. Workloads,
round counts, deterministic random data and warm buffers match the AWS
integration comparison above. Checked-backend is not enabled for timing.

Review verified `complete: true`, the exact two revisions, all 1344 unique
operation/profile/size/sample/variant combinations, expected raw/encoded lengths
and iteration counts, positive elapsed times, zero timed allocations and NEON
capability for both builds. These are operator-supplied development results,
not an independently attested capture or signed release admission.

Each cell below is the range across four profiles of the median paired old/new
elapsed-time ratio. A ratio above one means faster; payload sizes are decoded
bytes. All seven pairs favor the new implementation in every measured case at
3072 bytes and above.

| Payload bytes | Canonical decode | Historical `decode_slice` | Canonical validate |
| --- | ---: | ---: | ---: |
| 0 | 0.893-0.921x | 0.787-0.807x | 0.826-0.889x |
| 3 | 0.882-0.927x | 0.939-0.992x | 0.870-0.912x |
| 32 | 0.956-0.992x | 0.997-1.013x | 0.908-0.965x |
| 1024 | 0.988-1.012x | 0.984-1.016x | 0.983-0.999x |
| 3072 | 1.543-1.557x | 3.432-5.039x | 2.215-2.285x |
| 4096 | 1.572-1.607x | 3.442-5.137x | 2.312-2.362x |
| 65536 | 1.572-1.651x | 5.854-7.425x | 2.378-2.415x |
| 1048576 | 1.616-1.644x | 6.510-7.844x | 2.393-2.433x |

Tiny-input regressions must not be dismissed as universal measurement noise:
all profiles' three-byte canonical decode and validation cases lose all seven
pairs. Canonical decode's median per-call times increase by about 1-3 ns across
0/3/32-byte cases; validation increases by about 0.75-1.9 ns. Historical empty
calls increase by about 0.8-1.1 ns. These cases remain below the vector threshold,
so the benchmark alone does not identify the cause. The 1 KiB results are near
parity with mixed pair directions. This evidence supports a bulk-throughput
improvement, not a claim of no performance regressions at any size. The
maintainer explicitly accepted this tiny-input tradeoff and authorized Commit
11; no threshold or runtime code was changed to address it after measurement.

The raw file is retained locally at
`target/release-evidence/2.1-commit10-neon/integrated-macos-public-api.json`,
SHA-256 `aafec88773c48db856d0b4844111322a895ed6a6554d31eb5c0e26ebc273188e`.

The maintainer also supplied a passing native Mac gate transcript containing
the new production-linked invalid-lane test in all four active/MSRV,
plain/checked configurations and production IR/assembly checks in three feature
configurations. External review explicitly accepted `199a125`, closing the
previous Low verification gap, and the maintainer reported GitHub green.
Assembly semantics checks target the active Rust 1.98.1 code shape; MSRV 1.90
has execution coverage, not equivalent assembly-level coverage. None of these
correctness results substitutes for the paired performance data above.
