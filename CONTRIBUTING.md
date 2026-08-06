# Contributing

`base64-ng` is security-sensitive infrastructure code. Contributions should keep the crate small, boring, and easy to audit.

## Ground Rules

- Keep the dependency graph at zero external crates unless a dependency has written justification in the change.
- Prefer `core`, `alloc`, and `std` over helper crates.
- Keep scalar code safe Rust only.
- Keep unsafe code out of the crate until SIMD work starts, and then isolate it under a dedicated module.
- Preserve `no_std` support.
- Keep strict decoding as the default.
- Make legacy compatibility explicit and opt-in.

## Before Sending Changes

For a full local setup after reinstalling Rust, install the cross targets and
optional deep-check tools:

```sh
rustup target add aarch64-unknown-linux-gnu wasm32-unknown-unknown thumbv7em-none-eabihf
cargo install --locked cargo-nextest
cargo install --locked cargo-fuzz
cargo install --locked kani-verifier
```

Run:

```sh
scripts/checks.sh
```

For release-facing changes, run:

```sh
scripts/stable_release_gate.sh
```

For fuzz harness changes, run:

```sh
scripts/check_fuzz.sh
```

The standard checks include:

- `cargo fmt --all --check`
- release metadata validation
- zero-external-dependency validation
- clippy with warnings denied
- tests under default, all-features, and no-default-features
- docs build
- `cargo deny check`
- `cargo audit`
- `cargo license --json`
- fuzz-only dependency checks when `fuzz/` is present

## Dependency Additions

Dependency additions are rejected by default. If a change needs one, include:

- why `core`, `alloc`, or `std` is not enough
- whether it is runtime, dev-only, fuzz-only, bench-only, or CI-only
- the full transitive dependency impact
- license and advisory status
- why the dependency can remain optional

Do not add git dependencies.

Fuzz-only dependencies must stay under `fuzz/`, must not be included in the
published crate package, and must pass `scripts/check_fuzz.sh`.

## Testing Expectations

Narrow changes need focused regression tests. Shared behavior, parser/decoder logic, in-place operations, and public APIs need broader tests across padded, unpadded, standard, and URL-safe engines.

For future SIMD work, every fast path must prove equivalence to the scalar path before it can be enabled by default.

## Commit Policy

Commit completed, verified units of work. Leave pushing to maintainers.

The 2.0 implementation follows the exact numbered subjects in
[`2.0.0-release-plan.md`](docs/2.0.0-release-plan.md). One commit implements one
checkpoint. Pentest remediations use the same number plus a letter suffix;
pushed or reviewed checkpoints are never amended. Every checkpoint passes its
local verification before the next number starts. External pentests may cover
contiguous checkpoint batches, but all checkpoints require PASS coverage
before the final release. See
[`docs/2.0_GOVERNANCE.md`](docs/2.0_GOVERNANCE.md) for the frozen acceptance
and evidence rules.
