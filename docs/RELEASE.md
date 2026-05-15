# Release Checklist

This checklist is for maintainers preparing a `base64-ng` release.

## Preflight

- Confirm `Cargo.toml` has the intended version.
- Confirm `Cargo.toml` repository and homepage point to `https://github.com/valkyoth/base64-ng`.
- Confirm `CHANGELOG.md` has a section for that version.
- Confirm `license = "MIT OR Apache-2.0"` remains unchanged.
- Confirm `LICENSE-MIT` and `LICENSE-APACHE` are present at the repository root.
- Confirm no unwanted dependencies were added.

## Required Gate

Run:

```sh
scripts/stable_release_gate.sh release
```

The release gate covers:

- formatting
- metadata
- documentation version consistency checks
- packaged release script presence, executable-bit, and shebang validation
- dependency graph
- packaged dependency admission policy
- reserved feature placeholder checks with inert-feature and per-feature
  dependency graph validation
- fuzz-only dependency checks when `fuzz/` is present
- clippy
- feature-mode tests
- Miri no-default-features tests when nightly Miri is installed
- docs
- packaged async admission policy while the `tokio` feature remains inert
- installed cross-target `no_std` checks
- reserved SIMD feature-bundle compile checks for AVX2, AVX-512 VBMI,
  SSSE3/SSE4.1, NEON, and wasm `simd128` when the corresponding Rust targets
  are installed
- cargo-deny policy
- RustSec audit
- license inventory
- dudect-style timing harness compile and dependency checks
- constant-time assembly evidence generation
- SBOM generation
- reproducible package/build check

## Local Toolchain Setup

Install the cross targets used by `scripts/check_targets.sh` and CI:

```sh
rustup target add aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf
```

Install release and deep-check tools:

```sh
cargo install --locked cargo-audit
cargo install --locked cargo-deny
cargo install --locked cargo-license
cargo install --locked cargo-sbom --version 0.10.0
cargo install --locked cargo-nextest
cargo install --locked cargo-fuzz
cargo install --locked kani-verifier
```

Verify installation:

```sh
cargo audit --version
cargo deny --version
cargo license --version
cargo sbom --version
cargo nextest --version
cargo fuzz --version
cargo kani --version
```

The release gate detects these as Cargo subcommands, for example `cargo
nextest --version`, not by looking for standalone binaries named
`cargo-nextest`, `cargo-fuzz`, or `cargo-kani` on `PATH`.

Optional local timing evidence for the constant-time-oriented decoder can be
collected with:

```sh
BASE64_NG_RUN_DUDECT=1 scripts/check_dudect.sh
```

## Miri Setup

Miri is a nightly Rust component. Install it with:

```sh
rustup toolchain install nightly --component miri
cargo +nightly miri setup
```

Kani may need a one-time setup after `cargo install --locked kani-verifier`:

```sh
cargo kani setup
```

Kani bundles its own Rust compiler. If that compiler is older than this crate's
`rust-version`, `scripts/check_kani.sh` records an explicit skip until a newer
Kani release supports the pinned toolchain.

The local release gate runs:

```sh
scripts/check_miri.sh
```

If nightly Miri is not installed, the gate prints an explicit skip message.
`scripts/check_miri.sh` covers the no-default-features scalar surface and the
all-features alloc/stream surface. The large deterministic sweep tests remain
part of the normal stable test suite, but are ignored under Miri because Miri
interprets code and those sweeps are not practical there.

## Evidence

Release evidence is generated under:

```text
target/release-evidence/
```

Expected artifacts:

- `base64-ng.spdx.json`
- `base64-ng.cyclonedx.json`

The published crate package includes the core release/check scripts, Rust
toolchain pin, and cargo-deny policy so the documented gate can be inspected
with the packaged source.

Fuzz-only dependencies are checked separately with:

```sh
scripts/check_fuzz.sh
```

The `fuzz/` package is not part of the published crate. Corpus admission rules
are documented in `docs/FUZZING.md` and checked by
`scripts/check_fuzz_corpus.sh`.

Run the streaming fuzz smoke when changing stream state machines:

```sh
cargo +nightly fuzz run stream_chunks -- -runs=1000
```

Review generated local corpus files before committing. Commit only small,
non-sensitive inputs that preserve a regression, protocol boundary, or edge
case not already represented by deterministic tests.

Reserved SIMD feature bundles are checked with:

```sh
scripts/check_simd_feature_bundles.sh
```

Capture local backend evidence with:

```sh
scripts/check_backend_evidence.sh
```

The script writes a backend evidence manifest under
`target/release-evidence/backend/` with toolchain metadata, command status, and
checksums for the captured runtime backend report and inactive SIMD prototype
equivalence output.

## Publish

After the release gate passes:

```sh
cargo publish --dry-run
cargo publish
```

The dry run is intentionally kept as a manual publish preflight rather than
part of `scripts/stable_release_gate.sh`, because it updates the crates.io index
and may require network access.

Create and push the git tag only after the published crate is verified.

## Notes

Optional gates may be skipped until their harnesses exist:

- nextest
- Miri when the local nightly Miri component is not installed
- cargo-fuzz
- Kani

When these become active release requirements, update this checklist and `scripts/stable_release_gate.sh` together.
