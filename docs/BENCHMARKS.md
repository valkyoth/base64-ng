# Benchmarks And Performance Evidence

`base64-ng` keeps performance tooling isolated in `perf/` so the published core
crate remains zero-dependency. Commit 3 of the 2.0 plan replaces one-shot
timings with retained schema-versioned evidence.

## Comparison Scope

The harness measures caller-owned slice encode and strict decode for:

- Standard and URL-safe alphabets.
- Required canonical padding and explicitly unpadded forms.
- Boundary lengths around 12/16, 24/32, and 48/64-byte SIMD blocks, plus 1 KiB
  and 64 KiB throughput cases.
- Production auto dispatch, scalar, and each admitted backend that the current
  processor can execute safely.
- Exact-pinned `base64 0.23.0` and `base64ct 1.8.3`.

Comparison crates are included only for canonical valid inputs where alphabet,
padding, input, output, and caller-owned-buffer semantics match. Results do not
claim equivalence for malformed diagnostics, constant-time behavior, secret
retention, wrapping, streaming, or allocation helpers.

## Standard Check

Compile, test correctness with and without SIMD, audit dependencies, and check
the harness without running a timing campaign:

```sh
scripts/check_perf.sh
```

Run a complete campaign:

```sh
BASE64_NG_RUN_PERF=1 scripts/check_perf.sh
```

Useful controls:

```sh
BASE64_NG_RUN_PERF=1 \
BASE64_NG_PERF_CAMPAIGN_ID=my-host-commit \
BASE64_NG_PERF_SAMPLES=5 \
BASE64_NG_PERF_TARGET_BYTES=4194304 \
BASE64_NG_PERF_EVIDENCE_DIR=target/release-evidence/perf \
scripts/check_perf.sh
```

The exact-backend entry points exist only under the build cfg
`base64_ng_perf_evidence`. They check CPU availability before invoking an
implementation and are absent from normal crate builds. This prevents a
benchmark from claiming that a tier ran merely because a higher tier was
selected by production dispatch.

## Artifact Contract

The generated directory contains:

| Artifact | Contents |
|---|---|
| `environment.json` | CPU model, microcode, OS, kernel, Rust/Cargo versions, target, flags, governor, sample count, campaign size |
| `availability.csv` | Complete backend inventory and host availability |
| `raw-run-1.csv`, `raw-run-2.csv` | Raw sample durations and throughput |
| `summary.csv` | Median, minimum, and maximum throughput |
| `admission.csv` | Exact-backend ratio to scalar and admission status |
| `resources-default.csv` | Fixed staging bounds, adapter sizes, pending memory |
| `resources-no-simd.csv` | Same resource schema without the SIMD feature |
| `binary-resources.csv` | Release library bytes and symbol counts across feature sets |
| `MANIFEST.txt` | Thresholds, interpretation notes, and checksums |

`scripts/validate_perf_evidence.py` rejects changed headers, surplus CSV
cells, unknown or unpinned engines, dirty or incomplete Git provenance, missing
backend/profile/operation/length combinations, missing or extra sample
indexes, unsafe textual cells, non-finite measurements, and allocations in
measured slice operations. Standalone comparison and derivation commands load
the complete evidence bundle; there is no partial-matrix mode. Retained
summary and admission rows must recompute exactly from the raw runs, and binary
resource rows have a complete fixed feature-set inventory. The validator
derives the exact expected matrix from the complete backend-availability
inventory and fixed benchmark lengths. It also requires two same-host runs to
contain the same matrix and remain within a deliberately wide `0.50..2.00`
ratio envelope. The wide envelope detects broken campaigns; it is not a
precision-performance claim.

Campaign generation records the full clean source commit before any build,
requires environment capture to observe that same commit, and rechecks both
`HEAD` and worktree cleanliness after measurement and before writing the
manifest. Any observed source drift aborts the campaign.

Exact-backend rows below `0.95` of the matching scalar median are marked
`non-admissible-below-scalar`. That label is evidence for review, not an
automatic change to production dispatch. Backend admission or removal remains
a separate security and correctness decision.

The backend-specific review checklist remains
[`SIMD_ENCODE_ADMISSION_DRAFT.md`](SIMD_ENCODE_ADMISSION_DRAFT.md). Benchmark
evidence satisfies only its performance portion.

The original Commit 3 AMD Ryzen 9 9950X3D campaign was invalidated because it
was captured while the Commit 3 implementation was uncommitted and therefore
was not bound to the source named by its environment record. See
[`../performance-baselines/INVALIDATED_COMMIT_3.md`](../performance-baselines/INVALIDATED_COMMIT_3.md).
It admitted no backend. No number from that campaign remains eligible for a
release or admission claim. Its clean-source replacement is retained under
[`../performance-baselines/commit-5-correction-amd-9950x3d-linux/`](../performance-baselines/commit-5-correction-amd-9950x3d-linux/)
and is bound to the signed corrective source commit named in its environment
record.

## Resource Interpretation

Allocation counts are observed around one prepared caller-owned slice
operation and must remain zero. Binary size and demangled `base64-ng` symbol
counts are recorded for default, no-default, `secrets`, and `checked-backend`
feature sets.

Stack records are reviewed source bounds for fixed internal staging arrays:
768 bytes of input staging and 1024 bytes of output staging. Adapter object
sizes and maximum pending output capacities are measured with `size_of` and
public capacity methods. These values do not claim to measure the complete
dynamic call-chain stack, which depends on compiler, target, inlining, and
caller composition.

## Retained And Community Evidence

The archive contract is documented in
[`../performance-baselines/README.md`](../performance-baselines/README.md).
Every submission is local to its exact hardware and software environment.
Release notes may cite a result only with its source commit, environment,
commands, raw samples, and manifest.

Generation fails before measurement unless `git status --porcelain=v1
--untracked-files=all` is empty. The environment and manifest record the full
`HEAD^{commit}` identifier and `source_status=clean`. Retained evidence is
committed only in a later commit so its recorded source remains the exact clean
tree that executed the campaign.

Performance numbers are release notes evidence only when all retained
environment, correctness, reproducibility, and admission records are present.
Correctness runs before and after every timing campaign. A performance result
never substitutes for differential, Miri, Kani, fuzz, assembly, or backend
admission evidence.
