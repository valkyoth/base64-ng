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
