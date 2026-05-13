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
