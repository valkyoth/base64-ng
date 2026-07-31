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

Only canonical valid Standard and URL-safe padded/unpadded slice operations are
compared with exact-pinned `base64 0.23.0` and `base64ct 1.8.3`. The archive
does not present malformed-input, constant-time, wrapped, streaming, or secret
semantics as equivalent cross-crate comparisons.

The recorded stack values are reviewed source bounds for fixed staging arrays,
not dynamic whole-call-chain stack measurements. Allocation counts cover the
measured caller-owned slice operations. Community submissions must preserve
the generated files without hand editing and identify the source commit.

Transient compilation directories remain under `target/release-evidence/` and
must not be copied into a retained or submitted evidence directory.
