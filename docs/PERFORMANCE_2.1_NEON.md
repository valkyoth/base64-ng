# Commit 10 NEON Candidate Measurements

This is a bounded development experiment, not release admission or a general
performance guarantee. The NEON candidate is test-only. Existing production
NEON dispatch and its health/checked/static contracts are unchanged.

## Scope And Method

Measured on 2026-10-01 on AWS AArch64 Linux, four-core ARM Neoverse-V2,
Rust 1.98.1, release mode, `--no-default-features --features std,simd`. Source
is the Commit 10 working tree based on accepted
`d45a48b26507220ef8a0763dadc235ab0f139c20`, with the shared strict scalar tail
validator (not the earlier exploratory reference-tail implementation).

Seven samples rotate three complete operations in the same process: canonical
public Auto, canonical public ScalarReference, and the private NEON candidate.
The candidate includes family selection, vector prefix and scalar tail
validation, preflight/capacity checks, complete output writing and register
cleanup. Small inputs without a complete interior block do not execute NEON.
Inputs and outputs are allocated before timing. An independent RFC oracle
provides all four Standard/URL-safe padded/unpadded fixtures; outputs are checked
before and after each timed sample, with success and length checked every call.
Rounds are 10000 for 0/3/32-byte payloads, 500 for 1024 bytes, 32 for 65536 bytes,
and 4 for 1048576 bytes. The benchmark prints 504 sample records.

## Results

Standard padded median nanoseconds per complete decode; sizes are decoded
payload bytes. Speedup is Auto time divided by candidate time.

| Payload bytes | Public Auto | ScalarReference | NEON candidate | Speedup |
| --- | ---: | ---: | ---: | ---: |
| 0 | 22.95 | 38.05 | 16.32 | 1.41x |
| 3 | 46.29 | 201.94 | 20.58 | 2.25x |
| 32 | 63.08 | 1638.16 | 40.72 | 1.55x |
| 1024 | 841.09 | 49537.67 | 691.81 | 1.22x |
| 65536 | 51819.94 | 3109637.03 | 43003.88 | 1.21x |
| 1048576 | 841824.50 | 49482517.50 | 674575.50 | 1.25x |

Across all four profiles, 64 KiB speedup is 1.205-1.207x and 1 MiB speedup is
1.243-1.248x. These warm-buffer, fixed-pattern results do not establish a
crossover, cold-cache behavior, invalid-input throughput, or an Apple Silicon
result. The private candidate has no production backend health-selection
overhead yet; small-input numbers in particular are not promised public API
improvements. `ScalarReference` is the canonical policy, not the historical
Engine decoder. No comparison with other libraries is implied.

The raw final samples and native/QEMU gate logs are retained locally under
`target/release-evidence/2.1-commit10-neon/`; this is not a signed release bundle.

## Native Reproduction

On Apple Silicon and AWS ARM separately, from the same reviewed commit:

```sh
git status --porcelain
git rev-parse HEAD
rustc -Vv
sh scripts/check-2.1-neon-validation.sh
cargo test --locked --release --no-default-features --features std,simd \
    --lib v2::ordinary_decode::neon_candidate::benchmark::same_process_decode_comparison \
    -- --ignored --exact --nocapture --test-threads=1
```

Retain the commit, compiler, machine model, gate output and all benchmark rows.
The gate installs missing pinned/MSRV toolchains and targets and runs both
compiler feature matrices. Linux also runs guard pages. The same assembly
checker accepts ELF and Mach-O syntax and verifies production exclusion.
Apple Silicon native results remain pending. The candidate must acquire the
shared production health KAT, quarantine/recovery and checked-output comparison
before default routing can change, followed by native regression tests.

On a non-ARM Linux host with QEMU installed, use
`sh scripts/check-2.1-neon-validation.sh --qemu` for functional tests only.
Without that option the gate cross-compiles and checks codegen, explicitly
reporting that execution was not performed. QEMU timings are not hardware
performance evidence.
