# 2.1 Ordinary Policy Review

Commit 25 is in progress. This record separates policy decisions from final
release evidence. No new default, ISA, or threshold is admitted by a benchmark
script returning successfully. Native macOS capture and review of the complete
Linux captures remain pending; do not treat this checkpoint as release approval.

## Proposed Disposition

Retain the ordinary routes introduced and tested in Commits 5-20, subject to the
complete-operation measurements below. Preserve `ScalarReference` as an explicit
independent validator; it does not imply a scalar writer. `checked-backend`
continues independent reference validation and output comparison. Builds without
`simd`, custom alphabets, relaxed policies, secret/CT APIs, health quarantine and
transactional error contracts are not changed by this checkpoint.

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
  Corrected-source captures and full result disposition remain required.
- Native macOS, control comparisons and result disposition are pending.
- Final frozen-source correctness/fuzz/hardware campaigns remain Commit 27.
