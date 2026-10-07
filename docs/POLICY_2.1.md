# 2.1 Ordinary Policy Review

Commit 25's implementation and development policy review passed external retest
and CI through `7bcdaf0`. This record separates the selected 2.1 defaults from
final release admission. No ISA or threshold is admitted merely because a
benchmark script returned successfully. Frozen-source release campaigns remain
Commit 27; this checkpoint is not release approval.

## Selected Disposition

Retain the ordinary routes and size rules introduced and tested in Commits 5-20,
with the measured tradeoffs below. Do not add per-CPU tuning from these few hosts
or raise a shared floor to conceal an unfavorable cell. Preserve
`ScalarReference` as an explicit
independent validator; it does not imply a scalar writer. `checked-backend`
continues independent reference validation and output comparison. Builds without
`simd`, custom alphabets, relaxed policies, secret/CT APIs, health quarantine and
transactional error contracts are not changed by this decision. No new runtime
change accompanies the final policy record.

Automatic AVX-512 **decode** remains excluded. The fast AVX-512 validator is a
test-only candidate, not a health-gated complete public path. Results from the
older exact/static decoder, or from a kernel/prototype, cannot authorize turning
it on for public automatic calls. Existing automatic AVX-512 **encode** is a
separate policy. No SVE, new RVV CPU model, Windows ARM64 or new ISA is admitted.

Existing size rules, recorded before examining this capture:

| Route | Size rule |
| --- | --- |
| Shared ordinary x86/wasm decode | At least 512 encoded input bytes, then backend feature/health checks |
| Shared ordinary NEON decode | At least 4096 encoded input bytes |
| Shared ordinary RVV decode | At least 1024 interior encoded bytes after reserving the final quantum; current CPU admission still applies |
| x86 encode | SSSE3/SSE4.1 from 12 raw bytes, AVX2 from 24, AVX-512 from 192 |
| NEON/RVV encode | 192/384 raw bytes respectively |
| Canonical ordinary encode integration | At least 192 raw bytes, then the existing backend policy |
| Incremental encode | At least 192 aligned input and 256 available output bytes |
| Incremental decode | At least 516 input and 384 output bytes; at least 512 interior bytes, then shared backend eligibility |
| In-place decode | Shared whole-input preflight; disjoint scratch chunks of at most 1024 encoded bytes |

The shared ordinary floors are not replacements for historical/exact backend
thresholds. Final padding, partial quanta and canonical tail bits remain scalar.
Adapters inherit ordinary state-machine eligibility and retain their own framing,
backpressure and progress contracts. A whole-message speedup does not imply an
improvement with one-byte fragments or on each incremental call.

## Measurements

The revision capture transplants one current public-API harness onto signed
`v2.0.4` (`816da2e1e4a66c913057c86d149068f1c88776bf`) and the accepted candidate.
Both use the same pinned compiler, dependency versions and feature combination;
only local package versions in the harness lockfile are adjusted. Every timed
operation checks independent oracle output before/after measurement. Malformed
results must be errors, diagnostics/destination behavior are compared, and
allocations are counted rather than hidden.

`scripts/public_api_policy.py` fixes the matrix before capture: all four strict
profiles, small and bulk inputs, x86/NEON crossover sizes, empty/tail inputs,
cold first calls, early/interior/late rejection and fragmented/backpressured
incremental/adapter calls. Each cell has 15 alternating baseline/candidate pairs.
Cold calls execute once in a fresh process. Warm rounds are bounded and longer
than the exploratory smoke runner to reduce timer quantization.

Statistics remain those in `public_api_baseline.py`: median paired ratios,
10% relative-MAD noise screen, a 5% practical band and a paired sign test.
Per-cell signals are not simultaneous confidence claims across the whole matrix.
Retain noisy/inconclusive/regressing cells, repeat relevant questionable cells,
and review absolute latency as well as ratios. No threshold is automatically
lowered to make the results look better.

Same-source controls separately compare reference/automatic and
historical/canonical paths, pinned `base64 0.23.0` and `base64ct 1.8.3` valid-input
operations, and available exact AVX2/AVX-512 paths. The `base64ct` constant-time
contract differs from ordinary decoding; its timing is context, not justification
to weaken a secret-processing contract. No comparison to unpinned competitor
HEAD or to somebody else's peak-throughput chart is made.

All throughput uses decoded/raw payload GiB/s with encoded GiB/s retained
separately. Invalid input is latency-only. Ordinary output is not secret storage.

## Capture Commands

Linux uses the existing fail-closed Bubblewrap/cgroup runner:

```sh
python3 scripts/compare-2.1-public-api.py --policy-matrix --samples 15 \
  --features default simd adapters checked --output target/policy-revisions
python3 scripts/compare-2.1-public-api.py --policy-matrix --samples 15 --controls \
  --features simd checked --output target/policy-controls
```

The native macOS helper is an explicitly **trusted-code** operation, not an
untrusted-code sandbox or a fallback when Linux isolation is unavailable. It
extracts the fixed baseline and clean HEAD, transplants the same harness, builds
offline in fresh private directories without inherited compiler wrappers, and
records source/harness/compiler/binary hashes. Builds and measurements receive
a minimal environment without inherited credentials. Temporary build trees are
removed on success or exceptions. It does not include final hardware admission,
signed provenance, Miri, fuzzing, or host confidentiality guarantees.

On native Apple Silicon with Rust 1.99.0, Python 3.12+ and Xcode command-line
tools, from the clean pushed checkpoint:

Check `python3 --version` first. If it is older than 3.12, use an installed
`python3.12`, `python3.13` or `python3.14` for the capture command. The script
checks this before importing `tomllib` or starting a build; it does not install
Python or modify the system interpreter.

```sh
git pull --ff-only
cargo fetch --locked --manifest-path perf/public-api/Cargo.toml
python3 scripts/capture-2.1-macos-policy.py --trusted-revisions \
  --output "$HOME/base64-policy-macos"
tar -czf "$HOME/base64-policy-macos.tar.gz" -C "$HOME" base64-policy-macos
shasum -a 256 "$HOME/base64-policy-macos.tar.gz"
```

Use a fresh output name for a repeat; neither helper overwrites a previous
capture. Keep the original capture too. The low-level trusted native measurement
helper binds prebuilt executable hashes only; source/build provenance must
accompany it, as provided by the macOS wrapper's manifest.

## Evidence Status

- Commit 24 Windows ABI/callback follow-up passed external review at `1e7f1a3`;
  CI passed after the native Cargo-proxy fix at `b1ac680`.
- Windows' retained 15-pair same-source results remain [exploratory](WINDOWS_2.1.md),
  not a Windows 2.0.4-versus-2.1 comparison. Do not relabel them.
- The initial local Ryzen 9 9950X3D and AWS Neoverse-V2 old/new captures against
  `b1ac680` were stopped after finding a historical late-malformed regression.
  Their raw samples are diagnostic, incomplete evidence, not an accepted matrix.
  The local 64 KiB historical SIMD rows took roughly 2.5-2.7 ms per rejection
  versus 16-108 us at 2.0.4. Historical preflight unnecessarily computed canonical
  incremental diagnostics before discarding them and recovering historical
  errors. The follow-up uses the historical reference validator in shared
  preflight without changing grammar, quarantine or destination contracts.
  The fix at `efaecf8` passed maintainer-supplied external retest and CI;
  corrected-source captures below replace the interrupted `b1ac680` runs.
- Final frozen-source correctness/fuzz/hardware campaigns remain Commit 27.

## Retained Capture Review

The [compact review index](evidence/policy-2.1/summary.json) binds selected groups
and representative cells to their original manifests, raw data and hashes.
It is unsigned development evidence, not a hardware attestation. Full raw files
remain locally retained under `target/release-evidence/2.1-commit25`; they are
not bundled in the index or the published crate. Highlights show Standard padded
cases; the classifications cover all four profiles, including unfavorable cells.
Control classifications compare different operations on the same source, not
2.0.4 versus 2.1, and must not be counted as version regressions.

The candidate is `efaecf8f77a6a78f7cd467040dc6e0dacf4be2d8`, using Rust 1.99.0.
Each selected case has 15 alternating pairs. Local x86 and AWS ARM each cover
1,824 old/new cases plus 384/320 control cases respectively. Apple Silicon covers
1,824 old/new cases and 480 control cases. Its seven reports contain 69,120
samples; archive SHA-256 is
`ac238bb6d1bdec8bd26951865ecde6dd5aed666463d5b557d7f21fda884591fe`.
An additional 68-case focused repeat ran on each Linux host. Raw summaries were
recomputed, case coverage checked, and capture/harness hashes verified. No timed
allocation increases or oracle/diagnostic mismatches were reported.

The initial corrected Linux captures were interrupted at the maintainer's
request. Only complete feature groups contribute: x86 default/SIMD/adapters and
ARM default/SIMD. Remaining groups ran separately. ARM's first host had four
Neoverse-V2 CPUs, the replacement eight; results are not pooled across hosts.
The Mac reports Darwin 27.0.0/arm64 but does not record its chip model. Its binary
hashes agree with the build manifest; the executables were not supplied for
independent rebuilding. These limits are retained rather than inferred away.

Representative **64 KiB valid, warm, plain SIMD** results below give the range
over four profiles. Ratios compare complete operations with 2.0.4 on that host.
Historical decode is included to avoid presenting the much larger gain against
the old canonical state-machine validator as the gain against every old API.

| Host | Canonical encode speedup | Historical decode speedup | Historical decode payload GiB/s |
| --- | --- | --- | --- |
| Ryzen 9 9950X3D | 16.59-21.65x | 16.47-17.33x | 9.59-9.73 |
| AWS Neoverse-V2, first host | 1.94-1.99x | 4.01-4.71x | 1.39-1.40 |
| Apple Silicon, model unrecorded | 3.11-3.17x | 5.56-7.28x | 2.43-2.49 |

Default, SIMD and adapter old/new groups have no large (at least 64 KiB), valid,
warm regression signals on these named hosts. This does not mean every case
improves or that noisy cells passed. Cold calls, short fragments, rejected input,
and checked builds must be read separately. Compiler, CPU and timer conditions
limit extrapolation to other machines. No competitor-wide performance ranking
or new Windows, WASM, RVV or SVE performance claim follows from these Linux/Mac
captures; their previous scoped evidence and final campaign obligations remain.

## Explicit Tradeoffs

- **Small encode:** The focused x86 repeat improves 192/193-byte encoding but
  regresses at 383 bytes (about 31-43 ns extra), then improves at 384. Raising
  the floor to 384 would discard measured gains at smaller sizes. AWS regresses
  at 192/193 and 383/384 but improves at 767/768. Mac 383-byte encoding improves
  2.12-2.20x and 384 improves 2.66-2.81x. Retain the shared 192-byte integration
  floor instead of adding a CPU model table or blanket ARM cutoff. This accepts
  the bounded small-message cost; it is not a no-regression promise.
- **Checked encode:** Canonical/owned encoding now reaches the existing checked
  encoder, including independent scalar comparison and scratch wiping. The old
  canonical encoder stayed scalar. Large canonical encoding takes about
  2.2x/3.2x/2.8x the old time on x86/AWS/Mac respectively. Historical checked
  encoding is broadly unchanged in the large warm cases. Keep comparison and
  quarantine intact; ordinary SIMD throughput is not checked-mode throughput.
- **Checked incremental decode:** The focused x86 repeat is about 21-27 percent
  slower than 2.0.4. On AWS, 4,092/4,096-byte fragments improve roughly 64-67x,
  while 4,100/8,192-byte fragments are about 22-25 percent slower. Reserving the
  final quantum puts the former below the 4,096-byte NEON validation floor;
  the latter can select vector validation and its independent reference scan.
  Keep these checks, disclose the cliff, and do not recommend fragment sizing
  as a way to evade verification. The scalar route remains fully validating.
- **Rejected input and tiny calls:** Historical early/late malformed inputs can
  pay for fast rejection followed by original-diagnostic recovery. The discarded
  canonical scan was fixed, but multiple linear scans remain possible. Some
  empty, short append, and one-/seven-byte fragment cases also regress. Preserve
  original errors, mutation/progress contracts and quarantine, retain these
  results, and require application-level input ceilings at trust boundaries.

These are deliberate policy costs, not findings that Base64 grammar or memory
safety may be relaxed. `ScalarReference` remains a validation selection, not a
promise of a scalar writer or cheaper execution; it cannot disable checked
comparison. Ordinary APIs remain non-constant-time and non-wiping. Secret/CT
APIs are not changed. Automatic AVX-512 decode remains a no-go: the validation
prototype is test-only, and existing exact/static control measurements are not
evidence for an admitted health-gated complete fast path.

Commit 26 may freeze documentation, package contents and campaign tooling after
this checkpoint's retest/CI acceptance. Commit 27 must still collect the final
frozen-source evidence. This decision adds no mandatory benchmark job to normal
push CI and does not start those long campaigns early.
