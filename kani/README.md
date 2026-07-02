# Kani Proof Harnesses

Kani proof harnesses live behind `#[cfg(kani)]` in `src/lib.rs` so they verify
the same scalar implementation that normal users compile.

Run them with:

```sh
scripts/check_kani.sh
```

The release gate runs Kani automatically when `cargo kani` is installed and
this directory exists. If Kani's bundled Rust compiler is older than the
crate's pinned `rust-version`, the script records an explicit skip until a
compatible Kani release is available.

Current proof harnesses cover:

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

These are intentionally small bounded proofs. They complement Miri, fuzzing,
and deterministic integration tests; they are not a substitute for the future
`v1.0` goal of complete scalar in-place decode proofs.
