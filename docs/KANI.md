# Kani Verification Policy

`base64-ng` keeps Kani proof harnesses in the crate, but Kani execution depends
on the compiler bundled with the installed `cargo-kani` release.

## Current Status

- Local Rust toolchain: Rust `1.95.0`.
- Locally tested Kani: `cargo-kani 0.67.0`.
- Current result: `scripts/check_kani.sh` records an explicit skip because the
  installed Kani compiler is older than this crate's `rust-version`.

This is not a normal Cargo dependency-resolution issue. Kani runs are compiler-integration-sensitive because Kani is a verifier with its own compiler integration.
Updating the project to Rust `1.95` does not make an older Kani release
understand that toolchain automatically.

## How To Check

Run:

```sh
cargo kani --version
scripts/check_kani.sh
```

If the installed Kani compiler is compatible, `scripts/check_kani.sh` runs:

```sh
cargo kani --no-default-features
```

If Kani reports that its compiler requires an older Rust version than this
crate declares, the script prints a skip and exits successfully. The stable
release gate treats that as an explicit policy skip, not as completed formal
verification.

## Release Policy

For `v0.11`, the project must choose one of these outcomes:

- run Kani proofs with a compatible Kani release
- pin a documented compatible Kani workflow
- document a verifier exception and the replacement evidence required before
  `v1.0`

Replacement evidence may include Miri, deterministic exhaustive tests,
fuzz-corpus evidence, generated-code review, panic-policy validation, and local
invariant documentation, but it must be named explicitly.
A Kani skip is not the same as a proof.

## Upgrade Guidance

When a newer Kani release is available:

```sh
cargo install --locked kani-verifier
cargo kani setup
cargo kani --version
scripts/check_kani.sh
```

Do not lower `rust-version` only to make Kani run unless the whole crate still
passes the release gate and the MSRV change is intentional. The verifier should
follow the crate's supported Rust version, not the other way around.
