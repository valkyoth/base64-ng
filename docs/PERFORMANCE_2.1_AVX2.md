# Commit 7 AVX2 Candidate Measurements

This is a bounded development experiment, not release admission, a comparison
with other libraries, or a general hardware performance guarantee. The AVX2
candidate is test-only; public dispatch remains unchanged.

## Scope And Method

On 2026-09-30, measure the Commit 7 working tree based on accepted Commit 6
follow-up `4973047c2c808b3d95508bf4040fefcad3fa084b`, using an AMD Ryzen 9
9950X3D, Linux x86_64, Rust 1.98.1 / LLVM 22.1.8, release mode and all features.
Run three complete operations in the same process:

- Canonical public `decode_into` with `Auto` (Commit 5 portable table path).
- Canonical public `decode_into_with_validation` with `ScalarReference`.
- Internal AVX2 candidate, including feature checks, complete preflight,
  scalar remainder validation, capacity checks, output kernels and cleanup.

Use all four strict Standard/URL-safe padded/unpadded profiles. An independent
RFC oracle encodes a deterministic payload pattern. Output matches that oracle
before and after timing, and every timed call checks its success and length.
Allocate the input and output before timing. Seven samples rotate the order of
the three modes. Rounds per sample are 10000 for payloads up to 32 bytes, 500
for 1024 bytes, 32 for 65536 bytes and 4 for 1048576 bytes. Reproduce with:

```sh
cargo test --locked --release --all-features --lib same_process_decode_comparison \
    -- --ignored --nocapture --test-threads=1
```

The ignored benchmark requires native AVX2 and prints all 504 sample records.
It is deliberately separate from CI correctness gates. There is no manual loop
unrolling in this candidate and no claim of a measured unrolling benefit.

## Results

Standard padded median nanoseconds per complete decode call. Payload bytes are
decoded output bytes, not encoded input bytes. Speedup is Auto / candidate.

| Payload bytes | Public Auto | ScalarReference | AVX2 candidate | Speedup |
| --- | ---: | ---: | ---: | ---: |
| 0 | 12.16 | 19.71 | 19.02 | 0.64x |
| 3 | 28.97 | 90.71 | 87.79 | 0.33x |
| 32 | 38.91 | 859.01 | 257.64 | 0.15x |
| 1024 | 418.41 | 27065.24 | 533.56 | 0.78x |
| 65536 | 24341.75 | 1582322.66 | 6718.91 | 3.62x |
| 1048576 | 390896.00 | 25013731.00 | 100880.00 | 3.87x |

Across all four profiles, the large-message speedup ranges from 3.39x to 3.87x
at the two largest sizes. All measured small sizes (0 through 1024 bytes) are
slower than Auto. Retaining original scalar validation for the final 1-32 bytes
and the candidate's fixed overhead are significant for short messages. These
results support further integration work, not unconditional AVX2 selection.

## Limits And Follow-Up

This warm-buffer, fixed-pattern, single-host experiment has no statistical
admission threshold. It does not establish a crossover, worst-case latency,
cold-cache behavior, allocation counts, Intel performance, or improvements to
historical Engine, custom/relaxed, incremental, in-place, CT or secret APIs.
`ScalarReference` here is the canonical validation policy, not the historical
Engine API; all three modes include their own complete writing phase.

Malformed input is tested separately for exact diagnostic and destination
parity. Its performance is not measured here; rejection can still require a
full original-validator pass. Production promotion also requires replacing the
test-only kernel-disagreement assertion with reviewed fault handling, and
separate dispatch/admission evidence. See [the release plan](2.1.0-release-plan.md).

The final raw samples are retained locally at
`target/release-evidence/2.1-commit7-avx2/benchmark.log`; they are development
artifacts, not a signed release evidence bundle.
