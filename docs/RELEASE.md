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
- dependency graph
- clippy
- feature-mode tests
- Miri no-default-features tests when nightly Miri is installed
- docs
- cargo-deny policy
- RustSec audit
- license inventory
- SBOM generation
- reproducible package/build check

## Miri Setup

Miri is a nightly Rust component. Install it with:

```sh
rustup toolchain install nightly --component miri
cargo +nightly miri setup
```

The local release gate runs:

```sh
cargo +nightly miri test --no-default-features
```

If nightly Miri is not installed, the gate prints an explicit skip message. The
large deterministic sweep tests remain part of the normal stable test suite, but
are ignored under Miri because Miri interprets code and those sweeps are not
practical there.

## Evidence

Release evidence is generated under:

```text
target/release-evidence/
```

Expected artifacts:

- `base64-ng.spdx.json`
- `base64-ng.cyclonedx.json`

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
