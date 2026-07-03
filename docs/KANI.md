# Kani Verification Policy

`base64-ng` keeps Kani proof harnesses in the crate. Kani execution depends on
the compiler bundled with the installed `cargo-kani` release, so the supported
pairing is documented rather than assumed.

## Current Status

- Local Rust toolchain: Rust `1.90.0`.
- Locally tested Kani: `cargo-kani 0.67.0`.
- Current result: `scripts/check_kani.sh` verifies the full current
  no-default-features Kani harness set: 18 harnesses, 0 failures.

This is not a normal Cargo dependency-resolution issue. Kani runs are compiler-integration-sensitive because Kani is a verifier with its own compiler integration.
Updating the project to Rust `1.90` does not make an older Kani release
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

The source-level harness unwind bound is intentionally set to `70` because the
constant-time-oriented alphabet scanner runs a fixed 64-iteration loop. Kani's
global `--unwind` flag only applies to filtered single-harness runs, so the full
suite carries the bound on the harness attributes.

If Kani reports that its compiler requires an older Rust version than this
crate declares, the script prints a skip and exits successfully. The stable
release gate treats that as an explicit policy skip, not as completed formal
verification.

For opt-in expensive exploration, run:

```sh
scripts/check_kani_advanced.sh
```

The advanced script enables `--cfg base64_ng_kani_advanced`, prints each stage
before it starts, and runs advanced harness code generation by default. This
keeps the default advanced check bounded and verifies that the opt-in harnesses
still compile through Kani.

To run the broad public strict-decode no-panic proof, use:

```sh
BASE64_NG_KANI_PROVE_PUBLIC_SURFACE=1 scripts/check_kani_advanced.sh
```

The symbolic wrapped-decode harnesses are intentionally excluded from the
default advanced script because they can consume very large amounts of memory
and run for hours on local workstations. To run them manually, use:

```sh
BASE64_NG_KANI_EXPENSIVE_WRAPPED=1 scripts/check_kani_advanced.sh
```

These harnesses are not part of the mandatory release gate unless a future
release explicitly promotes them after runtime evidence shows they are
practical for CI.

## Harness Scope

Current harnesses cover:

- scalar `decode_chunk` output bounds and bit-packing agreement with decoded
  6-bit values
- unpadded scalar tail validation and decode output bounds
- scalar length-helper bounds
- bounded scalar encode/decode output-prefix bounds
- strict decode backend agreement with the scalar reference for one padded
  quantum
- in-place decode prefix bounds
- clear-tail cleanup behavior on decode failures
- constant-time-oriented validate/decode agreement for one quantum

The default advanced opt-in script additionally checks code generation for:

- broad no-panic exercise of selected public strict decode surfaces for an
  8-byte symbolic input
- strict wrapped decode output-prefix bounds for an 8-byte symbolic input
- strict wrapped clear-tail cleanup for an 8-byte symbolic input

The `BASE64_NG_KANI_PROVE_PUBLIC_SURFACE=1` harness additionally proves:

- broad no-panic exercise of selected public strict decode surfaces for an
  8-byte symbolic input

The `BASE64_NG_KANI_EXPENSIVE_WRAPPED=1` harness set additionally proves:

- strict wrapped decode output-prefix bounds for an 8-byte symbolic input
- strict wrapped clear-tail cleanup for an 8-byte symbolic input

## Historical v1.0.0 Verifier Exception

The initial `1.0.0` outcome accepted a documented verifier exception:

- keep all Kani harnesses in-tree and checked by `scripts/check_kani.sh`
- treat an incompatible Kani compiler as an explicit skip, not a proof
- require replacement evidence before release-sensitive changes are accepted
- do not claim Kani-complete or formally verified behavior in the `1.0.0`
  security contract

Replacement evidence for `v1.0` consists of:

- the full `scripts/checks.sh` gate
- Miri evidence from `scripts/check_miri.sh`
- bounded fuzz smoke evidence from
  `BASE64_NG_RUN_FUZZ_SMOKE=1 scripts/check_fuzz.sh`
- deterministic tests for scalar chunk packing, in-place decode, clear-tail
  cleanup, stream fail-closed behavior, profile behavior, and constant-time
  validate/decode agreement
- generated assembly evidence from `scripts/generate_ct_asm_evidence.sh`
- invariant documentation in [INVARIANTS.md](INVARIANTS.md)
- panic-policy enforcement through `scripts/validate-panic-policy.sh`
- release metadata, MSRV/toolchain, dependency, unsafe-boundary,
  constant-time-policy, and SIMD-admission validators

That exception was intentionally narrower than a formal proof. Current `1.0.x`
evidence now includes the bounded Kani harness set.
The stable `1.0.0` guarantee is the documented API and security contract backed
by release evidence, not a whole-crate or cryptographic formal-verification
claim.

## Future Verifier Admission

Other verifier or model-checking tools may be evaluated, but they are not
release-gate evidence until they have:

- a documented local install and CI path
- reproducible commands that work with the pinned Rust toolchain
- a scoped harness plan for scalar Base64 bit-packing and buffer bounds
- no runtime dependency impact on the published crate
- clear failure behavior in release scripts

Do not lower `rust-version` only to make Kani run unless the whole crate still
passes the release gate and the MSRV change is intentional. The verifier should
follow the crate's supported Rust version, not the other way around.

## Release Policy

For each future release, the project must choose one of these outcomes:

- run Kani proofs with a compatible Kani release
- pin a documented compatible Kani workflow
- document a verifier exception and the replacement evidence required before
  release

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

For future releases, revisit this document whenever the installed Kani release,
the pinned Rust toolchain, or the harness unwind policy changes.
