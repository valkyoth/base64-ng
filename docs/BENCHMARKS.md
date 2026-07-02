# Benchmarks

`base64-ng` keeps benchmark tooling isolated from the published crate so the
runtime crate remains zero-dependency.

The lightweight benchmark harness lives in `perf/` and compares `base64-ng`
standard padded encode/decode against the established `base64` crate using
deterministic input buffers. It deliberately uses `std::time` and
`std::hint::black_box` instead of a benchmark framework to keep the comparison
dependency graph small.

Compile and audit the benchmark harness:

```sh
scripts/check_perf.sh
```

Run the benchmark locally:

```sh
cargo run --release --manifest-path perf/Cargo.toml
```

The default perf build enables `base64-ng`'s `simd` feature. To force the
base64-ng scalar baseline, run:

```sh
cargo run --release --manifest-path perf/Cargo.toml --no-default-features
```

Capture benchmark output and a release-evidence manifest:

```sh
BASE64_NG_RUN_PERF=1 scripts/check_perf.sh
```

This writes:

- `target/release-evidence/perf/perf-output.csv` from the default perf build,
  which enables `simd` and records the active encode backend, active strict
  decode backend, and effective backend for each measured operation.
- `target/release-evidence/perf/perf-scalar-output.csv` from
  `--no-default-features`, which disables `simd` for the base64-ng scalar
  baseline.
- `target/release-evidence/perf/MANIFEST.txt` with toolchain metadata, command
  status, and artifact checksums.

Output is CSV:

```text
engine,operation,input_len,iterations,elapsed_ms,throughput_mib_s,effective_backend,active_backend,active_decode_backend,candidate_backend,detection_mode,target_arch,target_os
```

`active_backend` records the runtime backend selected by
`runtime::backend_report()` for the primary encode dispatch boundary.
`active_decode_backend` records
`runtime::backend_report().active_decode_backend()` for the normal strict
decode boundary. `effective_backend` records what the measured row actually
used: small encode or decode inputs that cannot fill the selected SIMD block
report the smaller fallback backend or `scalar`; rows for the external
`base64` crate report `external`.

Benchmark numbers are machine-local evidence, not portable guarantees. Release
notes should cite hardware, OS, Rust version, CPU governor, and command output
when publishing performance claims.

For a future SIMD encode admission release, use the benchmark record template in
[`SIMD_ENCODE_ADMISSION_DRAFT.md`](SIMD_ENCODE_ADMISSION_DRAFT.md). A speed
claim is not complete unless it names the active backend, target triple, CPU
model, command, scalar baseline, SIMD throughput, and raw artifact.

## Interpreting Results

The current scalar decoder uses arithmetic alphabet mapping instead of a large
decode lookup table. That keeps the default implementation aligned with the
side-channel hardening roadmap, but it is expected to trail highly optimized
table-based decoders on large buffers.

Treat scalar decode throughput as an optimization target for the current
development cycle, not as a release claim. Any future fast scalar or SIMD path
must preserve strict error indexes, canonical padding rejection, Miri
cleanliness, and scalar/SIMD differential test evidence.

The current `1.3.0` working line admits normal strict decode SIMD only for
Standard and URL-safe alphabet families. Wrapped, legacy, in-place,
custom-alphabet, and `ct` secret decode surfaces remain scalar and must not be
described by benchmark rows for the strict decode boundary.
