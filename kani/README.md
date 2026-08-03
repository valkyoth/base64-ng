# Kani Proof Harnesses

Kani proof harnesses live behind `#[cfg(kani)]` in `src/lib.rs` so they verify
the same scalar implementation that normal users compile.

Run them with:

```sh
scripts/check_kani.sh
```

The advanced release-host harness set is gated behind
`cfg(base64_ng_kani_advanced)` and can be run separately:

```sh
scripts/check_kani_advanced.sh
```

Run every required advanced harness with:

```sh
BASE64_NG_KANI_ALL_ADVANCED=1 scripts/check_kani_advanced.sh
```

Both runners apply explicit time and virtual-memory limits and write version,
command, harness, result, timing, peak-memory, unsupported-construct, and
checksum evidence under `target/release-evidence/kani/`.

The release gate runs Kani automatically when `cargo kani` is installed and
this directory exists. If Kani's bundled Rust compiler is older than the
crate's pinned `rust-version`, the script records an explicit skip until a
compatible Kani release is available.

The machine-checked [`harnesses.tsv`](harnesses.tsv) registry contains 43
normal, 19 advanced, and 6 exploratory harnesses. Current proofs cover:

- checked encoded length bounds for small symbolic lengths
- decoded capacity bounds for small symbolic lengths
- strict in-place decode returning only a prefix inside the caller buffer
- strict slice decode returning a written length inside the caller output
- strict decode backend agreement with scalar decode for one padded quantum
- strict clear-tail slice decode clearing caller output on error
- strict slice encode returning a written length inside the caller output
- strict in-place encode returning only a prefix inside the caller buffer
- strict clear-tail in-place decode clearing the caller buffer on error
- constant-time-oriented slice decode returning a written length inside the
  caller output
- constant-time-oriented clear-tail slice and in-place decode clearing caller
  buffers on error
- constant-time-oriented validate/decode agreement for one padded quantum
- all four strict 2.0 presets against an independent fixed-array RFC 4648
  model
- runtime alphabet and complete codec-policy invariants
- incremental and in-place refinement, rollback, overlap, and finalization
- portable SIMD arithmetic, masks, cursor bounds, and initialized output
- bounded secret release gates and absorbing failures
- an explicitly in-memory four-axis teardown, generation, journal, accounting,
  quarantine, and tombstone model

The two wrapped-input harnesses are exploratory because their unrestricted SAT
instances have demonstrated excessive resource use. They are not counted as
release proof. See
[`docs/2.0_FORMAL_VERIFICATION.md`](../docs/2.0_FORMAL_VERIFICATION.md) for the
exact claim and non-claim boundary.

These are intentionally small bounded proofs. They complement Miri, fuzzing,
and deterministic integration tests; they are not a substitute for the future
`v1.0` goal of complete scalar in-place decode proofs.
