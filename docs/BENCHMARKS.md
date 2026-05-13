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
