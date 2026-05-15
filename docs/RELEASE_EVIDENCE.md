# Release Evidence

`base64-ng` treats release evidence as part of the artifact, not as an informal
local habit. The release gate generates and verifies evidence that downstream
users can inspect before adopting a version.

Run the gate with:

```sh
scripts/stable_release_gate.sh release
```

The published crate package includes the core local gate scripts, Rust
toolchain pin, and cargo-deny policy referenced by this document, so downstream
reviewers can inspect the release checks alongside the source and documentation.

Install the optional targets and Cargo tools that make the release gate
exercise the deepest local paths:

```sh
rustup target add aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf
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
- packaged release script presence, executable-bit, and shebang validation
- zero-dependency policy check for the published crate
- packaged dependency admission policy for future external-crate review
- reserved feature placeholder checks for `tokio`, `kani`, and `fuzzing`,
  including inert-feature and per-feature dependency graph validation
- clippy with warnings denied
- default, all-features, and no-default-features tests
- doctests and documentation build
- `cargo deny check`
- `cargo audit`
- `cargo license --json`
- async admission documentation packaged while the `tokio` feature remains
  inert and dependency-free
- Miri through `scripts/check_miri.sh` when nightly Miri is installed,
  covering no-default-features scalar APIs and all-features alloc/stream APIs
- fuzz target compile check when `cargo-fuzz` is installed
- fuzz corpus policy validation for target-specific reviewed corpus inputs and
  release-blocking artifact cleanup
- isolated fuzz and performance harness dependency checks
- installed-target `no_std` checks for the reserved `simd` feature
- reserved SIMD feature-bundle compile checks for AVX2, AVX-512 VBMI,
  SSSE3/SSE4.1, NEON, and wasm `simd128` under `no_std` when the corresponding
  Rust targets are installed
- unsafe-boundary validation that confines `allow(unsafe_code)` to `src/simd.rs`
- unsafe-boundary validation that confines architecture intrinsics, CPU feature
  detection, and `target_feature` gates to `src/simd.rs`
- unsafe-boundary validation that requires inventory documentation for every
  SIMD-boundary unsafe function and a nearby `SAFETY:` explanation for every
  unsafe block
- panic-policy validation that fails on unreviewed non-test `panic!`,
  `unreachable!`, `.unwrap()`, or `.expect()` sites
- constant-time policy validation that keeps non-claim wording and
  generated-code review requirements in the documented release bar
- dudect-style timing harness compile and dependency checks, with timing runs
  opt-in for local release evidence
- constant-time assembly evidence generation for no-default-features and
  all-features release builds
- runtime backend report tests proving the public active backend remains scalar
  until an accelerated backend is explicitly admitted
- runtime backend policy tests for scalar execution and no-SIMD deployment
  assertions
- high-assurance scalar-only backend policy tests
- stable runtime enum string identifier tests for audit-friendly evidence
- stable key/value runtime report and policy-failure formatting tests
- constant-time-oriented clear-tail decode tests for success, malformed input,
  undersized output, and in-place cleanup
- stream encoder and decoder tests proving `finish()`, `into_inner()`, and
  adjacent-payload behavior remain intact after cleanup hardening
- stream fuzz coverage for chunked writers, fragmented reader sources, and
  adjacent framed payload boundaries
- Kani proofs through `scripts/check_kani.sh` when Kani is installed and its
  bundled compiler supports this crate's pinned `rust-version`
- bounded Kani coverage for constant-time-oriented decode result bounds,
  clear-tail cleanup on error, and validate/decode agreement
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

The `stream_chunks` fuzz target covers:

- chunked streaming encoders and decoders
- fragmented `DecoderReader` sources compared with slice decoding when payload
  boundary semantics match
- padded `DecoderReader` payloads followed by adjacent framed bytes, proving
  the reader leaves those bytes unread

Run a bounded local smoke test with:

```sh
cargo +nightly fuzz run stream_chunks -- -runs=1000
```

LibFuzzer may generate local corpus files under `fuzz/corpus/`; review them
before committing and discard accidental local corpus churn.

## SIMD Feature-Bundle Evidence

Reserved SIMD code must compile under the feature bundles that future admitted
backends will rely on. Check installed SIMD feature bundles with:

```sh
scripts/check_simd_feature_bundles.sh
```

This currently proves `no_std` reserved builds for AVX2, SSSE3/SSE4.1, the
AVX-512 Base64 candidate bundle (`avx512f`, `avx512bw`, `avx512vl`, and
`avx512vbmi`), NEON, and wasm `simd128` when the corresponding Rust targets
are installed.

Capture local runtime backend and prototype evidence with:

```sh
scripts/check_backend_evidence.sh
```

The script runs the runtime backend-report test and the gated SIMD prototype
scalar-equivalence tests with `--nocapture`. On CPUs with SSSE3/SSE4.1, AVX2,
or the AVX-512 candidate bundle, those prototype tests execute the inactive
prototype body and compare it against scalar output.

The release gate also runs:

```sh
scripts/validate-simd-admission.sh
```

That validator keeps active SIMD dispatch scalar-only until the release includes
the required scalar differential tests, fuzz evidence, unsafe inventory updates,
architecture evidence, benchmark evidence, and release-note wording.

## Constant-Time Timing Evidence

The release gate compiles the isolated dudect-style harness and checks its
dependency policy:

```sh
scripts/check_dudect.sh
```

Timing measurements are opt-in because shared CI runners are not stable enough
for reliable side-channel statistics:

```sh
BASE64_NG_RUN_DUDECT=1 scripts/check_dudect.sh
```

Archive the raw output with CPU, OS, Rust version, sample count, and command
line when using dudect-style evidence for a security review. This evidence is
empirical and does not replace generated-code review or Kani proofs.

Generate assembly artifacts for reviewer inspection with:

```sh
scripts/generate_ct_asm_evidence.sh
```

The script writes `target/release-evidence/asm/base64_ng-no-default-features.s`
and `target/release-evidence/asm/base64_ng-all-features.s`, plus
`target/release-evidence/asm/MANIFEST.txt` with rustc metadata, commands,
review focus, and artifact checksums.

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
