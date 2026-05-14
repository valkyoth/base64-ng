# Benchmarks

`base64-ng` keeps benchmark tooling isolated from the published crate so the
runtime crate remains zero-dependency.

The lightweight benchmark harness lives in `perf/` and compares scalar
`base64-ng` standard padded encode/decode against the established `base64`
crate using deterministic input buffers. It deliberately uses `std::time` and
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

Output is CSV:

```text
engine,operation,input_len,iterations,elapsed_ms,throughput_mib_s
```

Benchmark numbers are machine-local evidence, not portable guarantees. Release
notes should cite hardware, OS, Rust version, CPU governor, and command output
when publishing performance claims.

## Interpreting Results

The current scalar decoder uses arithmetic alphabet mapping instead of a large
decode lookup table. That keeps the default implementation aligned with the
side-channel hardening roadmap, but it is expected to trail highly optimized
table-based decoders on large buffers.

Treat scalar decode throughput as an optimization target for `0.4.1`, not as a
release claim. Any future fast scalar or SIMD path must preserve strict error
indexes, canonical padding rejection, Miri cleanliness, and scalar/SIMD
differential test evidence.
