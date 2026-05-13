# Release Checklist

This checklist is for maintainers preparing a `base64-ng` release.

## Preflight

- Confirm `Cargo.toml` has the intended version.
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

Create and push the git tag only after the published crate is verified.

## Notes

Optional gates may be skipped until their harnesses exist:

- nextest
- Miri when the local nightly Miri component is not installed
- cargo-fuzz
- Kani

When these become active release requirements, update this checklist and `scripts/stable_release_gate.sh` together.
