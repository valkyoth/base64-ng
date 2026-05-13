# Release Evidence

`base64-ng` treats release evidence as part of the artifact, not as an informal
local habit. The release gate generates and verifies evidence that downstream
users can inspect before adopting a version.

Run the gate with:

```sh
scripts/stable_release_gate.sh release
```

Install the optional targets and Cargo tools that make the release gate
exercise the deepest local paths:

```sh
rustup target add aarch64-unknown-linux-gnu wasm32-unknown-unknown thumbv7em-none-eabihf
cargo install --locked cargo-nextest
cargo install --locked cargo-fuzz
cargo install --locked kani-verifier
```

`cargo-fuzz` and Miri use nightly components:

```sh
rustup toolchain install nightly --component miri
cargo +nightly miri setup
```

## Required Checks

The release gate runs:

- formatting checks
- release metadata validation
- zero-dependency policy check for the published crate
- clippy with warnings denied
- default, all-features, and no-default-features tests
- doctests and documentation build
- `cargo deny check`
- `cargo audit`
- `cargo license --json`
- Miri when nightly Miri is installed
- fuzz target compile check when `cargo-fuzz` is installed
- isolated fuzz and performance harness dependency checks
- installed-target `no_std` checks for the reserved `simd` feature
- Kani proofs through `scripts/check_kani.sh` when Kani is installed and its
  bundled compiler supports this crate's pinned `rust-version`
- SBOM generation
- reproducible package/build check

## Generated Artifacts

Evidence is written under:

```text
target/release-evidence/
```

Expected files:

- `base64-ng.spdx.json`
- `base64-ng.cyclonedx.json`

The SBOMs describe the published crate dependency graph. The normal published
crate is zero-dependency; fuzz-only dependencies live under `fuzz/` and are
reviewed separately.

## Fuzz-Only Dependency Evidence

The fuzz harness is intentionally isolated from the published crate. Review it
with:

```sh
scripts/check_fuzz.sh
```

`fuzz/deny.toml` allows the NCSA license only for `libfuzzer-sys`. The root
`deny.toml` remains stricter for the published crate.

## Performance Evidence

The performance harness is intentionally isolated from the published crate.
Compile and review its dependencies with:

```sh
scripts/check_perf.sh
```

Run local scalar comparison measurements with:

```sh
cargo run --release --manifest-path perf/Cargo.toml
```

Performance numbers are release notes evidence only when paired with hardware,
OS, Rust version, CPU governor, and the exact command output.

## Reproducibility

The reproducible package/build check packages and verifies the crate twice and
compares the generated package file list. This catches accidental metadata,
include-list, or generated-file drift before release.

## Publishing

Before publishing:

```sh
scripts/stable_release_gate.sh release
cargo publish --dry-run
```

After `cargo publish`, verify crates.io metadata with:

```sh
cargo info base64-ng
```

Only tag and push once the published crate version is visible and correct.
