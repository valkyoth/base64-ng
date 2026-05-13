# Release Evidence

`base64-ng` treats release evidence as part of the artifact, not as an informal
local habit. The release gate generates and verifies evidence that downstream
users can inspect before adopting a version.

Run the gate with:

```sh
scripts/stable_release_gate.sh release
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
- Kani proofs when a `kani/` harness and `cargo-kani` are installed
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
