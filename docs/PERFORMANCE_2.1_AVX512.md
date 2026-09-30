# Commit 8 AVX-512 Candidate Measurements

This is a bounded development experiment, not release admission, a comparison
with other libraries, or a hardware-wide performance guarantee. Both vector
validation candidates remain test-only. Public automatic and exact/static
dispatch are unchanged.

## Scope And Method

On 2026-09-30, measure the Commit 8 working tree based on accepted Commit 7
follow-up `5add9f4b8d730453d8d3fdfb618a3bfc4fb9de7e`, using an AMD Ryzen 9
9950X3D, Linux x86_64, Rust 1.98.1 / LLVM 22.1.8, release mode and all features.
Compare three complete operations in the same process:

- Canonical public `decode_into` with `Auto` (Commit 5 portable table path).
- Internal AVX2 candidate from Commit 7.
- Internal AVX-512 candidate from Commit 8.

Both candidates include feature checks, complete semantic preflight, original
scalar remainder validation, capacity checks, writing and register cleanup.
An independent RFC oracle encodes a deterministic payload pattern for all four
strict Standard/URL-safe padded/unpadded profiles. Check output before and after
timing and verify each timed call's success and output length. Input/output
allocation is outside timing. Seven samples rotate the three modes' ordering.
Rounds per sample are 10000 for payloads up to 32 bytes, 500 for 1024 bytes,
32 for 65536 bytes and 4 for 1048576 bytes. Reproduce with:

```sh
cargo test --locked --release --all-features --lib avx512_same_process_decode_comparison \
    -- --ignored --nocapture --test-threads=1
```

The ignored benchmark requires native AVX2 and the complete AVX-512 VBMI
feature bundle. It emits 504 sample records and is not a CI speed assertion.
Other test/build workloads were finished before measurement.

## Results

Standard padded median nanoseconds per complete call. Payload bytes are decoded
output bytes, not encoded input bytes. Speedup is AVX2 time / AVX-512 time.

| Payload bytes | Public Auto | AVX2 candidate | AVX-512 candidate | Speedup vs AVX2 |
| --- | ---: | ---: | ---: | ---: |
| 0 | 11.40 | 16.45 | 20.55 | 0.80x |
| 3 | 28.96 | 74.11 | 76.19 | 0.97x |
| 32 | 37.00 | 270.61 | 847.84 | 0.32x |
| 1024 | 381.98 | 518.34 | 510.88 | 1.01x |
| 65536 | 23653.28 | 7021.03 | 4199.78 | 1.67x |
| 1048576 | 383063.75 | 107189.50 | 62447.50 | 1.72x |

Across all four profiles and the two largest sizes, AVX-512 is 1.46-1.72x
faster than the AVX2 candidate and 5.26-6.17x faster than public Auto in this run.
All tested small sizes through 1024 bytes remain slower than public Auto.
The 32-byte payload's encoding fits wholly in the AVX-512 scalar remainder,
while AVX2 already validates a vector prefix; the larger scalar remainder makes
that case especially unfavorable. At 1024 bytes, the two candidates are close
(0.99-1.03x), not evidence for a reliable AVX-512 advantage.

## Limits And Follow-Up

This short, warm-buffer, fixed-pattern experiment has no statistical admission
threshold. It cannot establish dispatch crossover sizes, cold-cache behavior,
frequency effects under sustained/mixed loads, Intel behavior, worst-case
latency, allocation counts or portability. It says nothing about historical
Engine, incremental, in-place, custom/relaxed, CT or secret performance. Rejected
inputs are tested for semantics separately, not benchmarked; they may need a
second full-input reference pass for exact errors.

Production promotion requires reviewed fault/quarantine handling in place of
the test-only kernel assertion. Automatic AVX-512 admission remains Commit 25's
separate decision. Wider vectors are not unconditionally faster, and these
measurements do not change any public dispatch threshold.

Raw samples are retained locally in
`target/release-evidence/2.1-commit8-avx512/benchmark.log`. They are development
artifacts, not a signed release evidence bundle.

## Pentest Hardening Recheck

The table above records the initial Commit 8 implementation. After the pentest
follow-up on `f3d9b23846b3c7c3fe501009ebbdb6fca5945c67`, the shared production
AVX-512 block loop validates its own geometry and derives arrays from checked
slices. First-block rejection now receives loop-level cleanup, and the candidate
wrapper's redundant cleanup was removed.

Repeating the same 504-record experiment on the same host/toolchain produced
Standard padded medians of 4311.56 ns (64 KiB) and 65115.00 ns (1 MiB) for the
hardened candidate, versus 7238.31 ns and 110124.75 ns for AVX2. Across all four
profiles at those two sizes, the AVX-512/AVX2 speedup remained 1.46-1.71x.
Small-message overhead relative to public Auto remains. This is a short
performance screen, not a statistically controlled before/after regression
bound or an admission decision. The fresh raw samples are retained separately
in `target/release-evidence/2.1-commit8-avx512/benchmark-pentest-followup.log`.
