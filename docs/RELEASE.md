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

`release` mode refuses pre-release Cargo versions such as `-alpha` builds.
Use `check` mode during pre-release development:

```sh
scripts/stable_release_gate.sh check
```

The release gate covers:

- stable-version enforcement when run in `release` mode
- formatting
- metadata
- documentation version consistency checks
- public API audit status checks
- packaged release script presence, executable-bit, and shebang validation
- dependency graph
- packaged dependency admission policy
- reserved feature placeholder checks with inert-feature and per-feature
  dependency graph validation
- fail-closed wasm wipe policy check and explicit
  `allow-wasm32-best-effort-wipe` opt-in build check
- fuzz-only dependency checks when `fuzz/` is present
- clippy
- feature-mode tests
- Miri no-default-features tests when nightly Miri is installed
- Miri evidence manifest generation when nightly Miri is installed
- all-features and no-default-features doctests
- all-features and no-default-features docs
- packaged async admission policy while the `tokio` feature remains inert
- installed cross-target `no_std` checks
- no-alloc portability smoke crate checks for installed Linux, FreeBSD, wasm32,
  ARM, and Cortex-M targets
- CI target-matrix no-alloc smoke checks for each installed `no_std` target
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

The no-alloc portability smoke crate checks the same installed target list with:

```sh
scripts/check_no_alloc_smoke.sh
```

Install release and deep-check tools:

CI and local release scripts use `scripts/ci_install_rust.sh`; that script uses rust-toolchain.toml as the single source of truth for the pinned stable Rust toolchain.

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

Kani bundles its own Rust compiler. The current supported local path is Rust
`1.90.0` with `cargo-kani 0.67.0`; `scripts/check_kani.sh` runs the
no-default-features harness set when that pairing is available. If a future
Kani/compiler pairing is incompatible with this crate's `rust-version`, the
script records an explicit skip rather than treating it as proof.
The Kani compatibility and verifier policy is documented in
[`KANI.md`](KANI.md).

The standard local gate runs isolated dudect, fuzz, and performance harness
compile/dependency checks without running timing campaigns, fuzz campaigns, or
benchmarks. The local release gate also runs:

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

Fuzz-only dependencies are included in the standard local gate and can also be
checked directly with:

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

Run the full stable release gate before creating the tag:

```sh
scripts/stable_release_gate.sh release
```

This is the expensive pre-tag gate. It includes Miri, Kani, generated assembly
evidence, SBOM generation, reproducibility checks, and the standard local gate.
If this fails, fix the release candidate before tagging.

After the full release gate passes, push the commit, wait for GitHub to become
green, then create and push the immutable release tag:

```sh
git tag -s v1.0.10 -m "base64-ng 1.0.10"
git push origin v1.0.10
```

Publish only from the tagged commit:

```sh
scripts/release_crates.py --check
scripts/release_crates.py --dry-run
scripts/release_crates.py
```

`scripts/release_crates.py` reads `release-crates.toml`, validates workspace
crate versions and dependency order, refuses real publishing unless `HEAD`
matches the `v<version>` tag, runs the standard local gate and
`cargo publish --dry-run` for each selected crate, publishes `base64-ng` first,
waits for crates.io visibility, and then publishes dependent companion crates
such as `base64-ng-sanitization` and `base64-ng-derive`, `base64-ng-serde`,
`base64-ng-bytes`, and `base64-ng-tokio`.

The publish helper intentionally does not rerun Kani by default. Kani belongs
to the pre-tag stable release gate so a verifier failure does not happen after
an immutable GitHub tag has already been created. If a release manager wants to
rerun the expensive gate immediately before publishing, use:

```sh
scripts/release_crates.py --full-gate
```

For manual fallback, publish the core package first, wait until crates.io serves
the new `base64-ng` version, then verify and publish the companion package:

```sh
cargo publish -p base64-ng --dry-run
cargo publish -p base64-ng
cargo package -p base64-ng-sanitization
cargo publish -p base64-ng-sanitization --dry-run
cargo publish -p base64-ng-sanitization
cargo package -p base64-ng-derive
cargo publish -p base64-ng-derive --dry-run
cargo publish -p base64-ng-derive
cargo package -p base64-ng-serde
cargo publish -p base64-ng-serde --dry-run
cargo publish -p base64-ng-serde
cargo package -p base64-ng-bytes
cargo publish -p base64-ng-bytes --dry-run
cargo publish -p base64-ng-bytes
cargo package -p base64-ng-tokio
cargo publish -p base64-ng-tokio --dry-run
cargo publish -p base64-ng-tokio
```

This order is required because companion crates depend on the same released
`base64-ng` version from crates.io while using a local path only during
repository development.

The publish sequence is intentionally kept out of
`scripts/stable_release_gate.sh`, because publishing updates the crates.io index
and requires release credentials. If publishing fails because of a crates.io or
credential issue after the tag exists, keep the tag and rerun the publish
helper. If publishing fails because the tagged source is wrong, do not move the
tag; cut a new patch release.

## Notes

Optional gates may be skipped until their harnesses exist:

- nextest
- Miri when the local nightly Miri component is not installed
- cargo-fuzz
- Kani

When these become active release requirements, update this checklist and `scripts/stable_release_gate.sh` together.
