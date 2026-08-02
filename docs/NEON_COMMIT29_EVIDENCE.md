# Commit 29 AArch64 NEON Evidence

Commit 29 replaces the earlier AArch64 block scaffolding with direct
little-endian NEON encode and strict-decode kernels for Standard and URL-safe
alphabet families.

## Admitted Scope

- direct 12-byte to 16-byte encode with exact 8+4-byte input reads
- direct 16-byte to 12-byte strict decode with validation before exact 8+4-byte
  output stores
- padded and unpadded scalar tails
- automatic `std` dispatch at measured conservative crossovers
- `StaticBackendToken` execution under `no_std`
- checked-backend comparison, quarantine, and per-operation reporting

Custom alphabets, big-endian AArch64, 32-bit ARM, CT secret decode,
line-ending insertion, and whitespace/line-ending compaction remain scalar.

## Required Real Devices

The checkpoint requires both:

1. Apple Silicon macOS, tested with `scripts/check_macos.sh`.
2. Server-class little-endian AArch64 Linux, tested with
   `scripts/check_aarch64_linux.sh`.

Run the performance campaign on each device:

```sh
BASE64_NG_RUN_COMMIT29_PERF=1 scripts/check-2.0-neon-hot-paths.sh
```

The gate writes `target/release-evidence/commit-29-neon.csv` and requires at
least three scalar and NEON samples for encode/decode, Standard/URL-safe,
padded/unpadded, and every recorded length. Automatic sizes at or above 192
raw bytes must exceed scalar by the configured ratio, which defaults to 1.02.

## Assembly Contract

`scripts/generate_neon_asm_evidence.sh` cross-generates fresh release assembly
and checks for:

- `uminv` all-lane validity reduction before strict-decode stores
- `tbl` byte permutation/compaction
- `bsl` vector alphabet selection
- an exact final four-byte lane store
- the reviewed vector-register cleanup sequence

Generated assembly is supporting compiler evidence. Real-device execution is
still required for correctness, ABI, and performance admission. Neither form
of evidence is a formal microarchitectural timing or data-remanence proof.

## External Evidence Record

Record the exact commit hash, CPU model, OS version, `rustc -Vv`, command, raw
CSV, and script result for both required systems before the final 2.0 release
gate accepts Commit 29.
