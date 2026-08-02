# Performance Evidence Archive

This directory defines the retained, machine-readable performance evidence
format used by the 2.0 development line. Evidence is local to the exact host,
toolchain, flags, and source commit named by its manifest. It is not a portable
performance guarantee.

Generate a complete submission:

```sh
BASE64_NG_RUN_PERF=1 \
BASE64_NG_PERF_EVIDENCE_DIR=target/release-evidence/perf \
scripts/check_perf.sh
```

Each submission contains:

- `environment.json`: CPU model and microcode, OS, compiler, target, flags,
  governor, sample count, and campaign size.
- `availability.csv`: every admitted backend and whether it could execute.
- `raw-run-1.csv` and `raw-run-2.csv`: raw samples for reproducibility review.
- `summary.csv`: medians and observed ranges.
- `admission.csv`: exact-backend ratios against scalar; rows below the retained
  `0.95` threshold are marked `non-admissible-below-scalar`.
- `resources-*.csv`: fixed staging bounds, adapter object sizes, and pending
  output capacities.
- `binary-resources.csv`: release artifact size and `base64-ng` symbol count
  for default, no-default, `simd`, `secrets`, and `checked-backend` feature
  sets.
- `MANIFEST.txt`: policy values and artifact checksums.

Generation is permitted only from a clean committed tree. `environment.json`
and `MANIFEST.txt` must identify the same full source commit and a clean source
status. The runner binds that commit before building and rejects observable
`HEAD` or worktree changes after measurement and around atomic manifest
finalization. The validator reconciles manifest metadata with
`environment.json`, requires the exact artifact inventory, and verifies every
digest. Hostile-concurrency collection additionally requires an immutable or
access-controlled detached checkout because ordinary worktree checks cannot
exclude a privileged concurrent writer.
Evidence artifacts are committed separately after capture; the artifact
commit is not substituted for the recorded measurement-source commit.

The Commit 34 focused x86 freeze campaign is retained under
`dispatch-commit-34-amd-9950x3d-linux/`. Unlike the complete cross-crate schema above,
that focused campaign measures exact production backends only. Its manifest,
raw CSV checksums, 15-sample median/sign-test policy, and automatic AVX-512
decode rejection are validated by `scripts/validate-2.0-dispatch-matrix.sh`.

Only canonical valid Standard and URL-safe padded/unpadded slice operations are
compared with exact-pinned `base64 0.23.0` and `base64ct 1.8.3`. The archive
does not present malformed-input, constant-time, wrapped, streaming, or secret
semantics as equivalent cross-crate comparisons.

The recorded stack values are reviewed source bounds for fixed staging arrays,
not dynamic whole-call-chain stack measurements. Allocation counts cover the
measured caller-owned slice operations. Community submissions must preserve
the generated files without hand editing and identify the source commit.
Campaign, run, feature-set, target, and other machine labels use the restricted
`[A-Za-z0-9][A-Za-z0-9._-]{0,63}` grammar. The validator requires the complete
Cartesian product of fixed lengths, operations, profiles, comparison engines,
and every backend marked available, with exact sample indexes
`0..sample_count`. It rejects surplus CSV cells and unsafe textual fields.
Timing rows must reconcile iterations with the campaign target, encoded
lengths with the selected Base64 profile, and throughput with the raw timing
fields. Comparison, summary, and admission commands accept only a complete
evidence directory. Retained summaries and admission tables must exactly
recompute from the raw runs, and resource and binary-resource evidence must
contain their complete fixed inventories.

Transient compilation directories remain under `target/release-evidence/` and
must not be copied into a retained or submitted evidence directory.
