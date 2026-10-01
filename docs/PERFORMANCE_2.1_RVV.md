# 2.1 RVV Validation Checkpoint

Commit 12 improves ordinary strict Standard/URL-safe decoding on the existing
exact Linux/SpacemiT X60 profile. This is a development checkpoint, not a release
seal, a general RVV admission, or a constant-time claim. External review and CI
remain pending. Secret/CT algorithms and hardware identity policy are unchanged.

## Implementation

Two stackless assembly leaves classify a readable byte span using variable VL,
unsigned alphabet ranges, and exact symbol masks. `vcpop.m == VL` is required
for every iteration; no inactive lane participates and there are no stores.
Every return clears the used vector registers. The closed safe boundary checks
exact-profile availability and geometry before assembly entry. The packing
wrapper independently classifies before invoking the existing sextet packer.

Shared preflight retains scalar tail grammar, exact reference diagnostics,
capacity-before-write, quarantine and complete scalar rewrite after failure.
The existing 1024-byte RVV crossover is applied to `len.saturating_sub(1) / 4 * 4`
after reserving the final complete or partial quantum. Valid unpadded inputs
first qualify at 1026 encoded bytes (1027 also qualifies); valid padded inputs
first qualify at 1028. Inputs of 1024 encoded bytes remain below the crossover.
No new automatic CPU profile, static deployment token, or unsafe public API
is added.

## Native Comparison

Host: BananaPi BPI-F3, SpacemiT X60, Linux 6.6.63, glibc 2.41, native RVV.
Both executables were cross-built with Rust 1.98.1 for
`riscv64gc-unknown-linux-gnu`, release optimization, `std,simd`, and only
`--cfg base64_ng_perf_evidence` in RUSTFLAGS. No candidate cfg or global `+v`
was used. The shared board was not frequency-locked or isolated; these are
exploratory measurements, not portable throughput promises.

The unmodified baseline is Commit 11 `d24f07cffcd42a19bbb7416c6bfe3ef24b481c40`.
The candidate is this checkpoint's implementation, measured before committing.
The existing `perf/public-api` harness independently checks decoded results
against its RFC 4648 oracle outside the timer. Both binaries report RVV
capability and zero timed allocations. Capability alone is not route proof;
native unit tests separately assert actual shared validator/writer execution.

`scripts/measure-2.1-rvv-public-api.py` collects seven alternating-order pairs
per case: canonical/historical caller-buffer decode, all four strict profiles,
and raw payload sizes 0, 3, 32, 768, 1024, 65536 and 1048576 bytes. It executes
trusted, locally built binaries only, without a sandbox. The general
untrusted-repository runner retains its separate sandbox requirements.

| Raw payload | Canonical speedup range | Historical speedup range |
| --- | --- | --- |
| 1 KiB | 1.29-1.32x | 1.67-2.54x |
| 64 KiB | 1.81-1.83x | 2.61-3.88x |
| 1 MiB | 1.68-1.69x | 2.50-3.71x |

Ratios are median paired baseline/candidate latency. All 64 KiB and 1 MiB
cases meet the harness's exploratory improvement criterion. Canonical URL-safe
unpadded at 1 KiB is inconclusive by its sign test despite the median gain.
The 0/3/32/768-byte canonical cases are within 5 percent. Some historical empty
and 3-byte cases regress: roughly 15-20 ns per call, with statistically flagged
regressions for Standard padded 3 bytes and URL-safe padded empty/3 bytes and
URL-safe unpadded empty. These remain scalar-sized calls, not faster RVV cases;
the change is not claimed to improve every size. No equivalent claim is made
for checked-backend throughput, cold-start KAT latency, or other boards.

Local raw result: `target/release-evidence/2.1-commit12-rvv/paired.json`.
SHA-256: `4c1fe672d8695cf834f823c8b5ee1e935aa350401a6db4a3df5836f9c9d7c39e`.
The JSON retains all 784 raw samples, host details and these executable hashes:

- Baseline: `9d6ef369fc8aba9ec307ab9f74d5fd6d02adb97feba94dc8501524b66e382e3f`.
- Candidate: `cf832674b7ab8b3eb591380a0da2cb6c7e769feebd6c4cd5ad2e865ff2782cc1`.

## Verification And Reproduction

Native cross-built execution passed: 189 active plain unit tests and 245
all-feature unit tests, explicit exact-profile admission, shared recovery/fault
tests on active and MSRV builds, and all five production-linked integration
tests under each compiler in both plain and checked configurations. Direct
candidate byte/VL/guard-page tests, thread switching and the explicit native
signal-frame test also passed. The native Rust 1.98.1 toolchain was not installed
on the board; compilation and assembly inspection ran on the development host.

The pentest boundary follow-up corrects documentation without changing runtime
routing. Native tests at 1024/1026/1027/1028 encoded bytes passed under Rust
1.98.1 and 1.90.0, plain and checked, with `BASE64_NG_REQUIRE_X60=1`. All four
strict profiles check actual validator/writer execution, reference-equivalent
results, malformed-tail diagnostics and unchanged rejected destinations.

Local workspace all-feature tests and Clippy passed, along with x86 public
decode, NEON QEMU and WASM active/MSRV gates. SVE-candidate and big-endian
PowerPC64 all-feature libraries cross-compiled under both compilers. These
cross-builds are not new SVE or big-endian native hardware evidence.

The focused gate checks Rust 1.98.1 and MSRV 1.90.0, no_std/standard and checked
features, the complete production assembly instruction/control-flow contract,
mutation rejection, direct QEMU VLEN 128/256 execution and no-V scalar fallback.
QEMU results explicitly do not authorize native admission.

```sh
sh scripts/check-2.1-rvv-validation.sh --qemu
# On the actual admitted Linux/X60 host:
sh scripts/check-2.1-rvv-validation.sh
```

Native execution may instead use cross-built test binaries. The ignored
`simd::rvv::ordinary::tests::rvv_native_exact_profile_is_required` test must run
explicitly and pass; production-linked `decode_validation` tests run with
`BASE64_NG_REQUIRE_X60=1`. Shared fault tests must exercise RVV, not silently
skip it. Candidate-only direct tests cover arbitrary byte lengths, every byte
in every lane/offset, guard pages, exact stores, and rejection before stores.
The existing native signal/context tests remain separate from QEMU evidence.

Build both trusted public harnesses with identical flags, transfer their binaries
and the following Python files, then run on the board:

```sh
python3 scripts/measure-2.1-rvv-public-api.py ./baseline-perf ./candidate-perf > paired.json
```

Keep `scripts/public_api_baseline.py` beside the measurement script. Cross-built
execution does not imply the board has a native Rust 1.98.1 installation.
The standard 2.1 final release gates still apply after integration.
