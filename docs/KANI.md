# Kani Verification Policy

`base64-ng` keeps Kani proof harnesses in the crate. Kani execution depends on
the compiler bundled with the installed `cargo-kani` release, so the supported
pairing is documented rather than assumed.

## Current Status

- Active release toolchain: Rust `1.98.1`.
- Kani verifier toolchain: Rust `1.90.0`.
- Locally tested Kani: `cargo-kani 0.67.0`.
- Current inventory: 43 normal, 19 advanced, and 6 exploratory harnesses.
- `scripts/check_kani.sh` verifies every normal no-default-features harness.
- `BASE64_NG_KANI_ALL_ADVANCED=1 scripts/check_kani_advanced.sh` verifies every
  required advanced harness on the release-evidence host. Exploratory
  harnesses never count as release proof.

This is not a normal Cargo dependency-resolution issue. Kani runs are compiler-integration-sensitive because Kani is a verifier with its own compiler integration.
Updating the active release toolchain to Rust `1.98.1`
does not make every Kani release understand that compiler automatically, so
Kani evidence records the exact verifier pairing separately from the normal
Cargo release toolchain.

## How To Check

Run:

```sh
cargo kani --version
scripts/check_kani.sh
```

By default, `scripts/check_kani.sh` runs through the documented
`1.90.0-x86_64-unknown-linux-gnu` toolchain. Override this only for verifier
experiments:

```sh
BASE64_NG_KANI_TOOLCHAIN=1.98.1-x86_64-unknown-linux-gnu scripts/check_kani.sh
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

For advanced code generation and selected proof groups, run:

```sh
scripts/check_kani_advanced.sh
```

The advanced script enables `--cfg base64_ng_kani_advanced`, prints each stage,
enables the `secrets` feature, and runs advanced harness code generation by
default. Both scripts apply explicit memory and per-harness time limits and
retain version, command, result, resource, warning, and checksum evidence under
`target/release-evidence/kani/`.

Run the complete required advanced set with:

```sh
BASE64_NG_KANI_ALL_ADVANCED=1 scripts/check_kani_advanced.sh
```

Run only the independent RFC 4648 refinements or assurance protocol model with:

```sh
BASE64_NG_KANI_PROVE_FINAL_CORE=1 scripts/check_kani_advanced.sh
BASE64_NG_KANI_PROVE_ASSURANCE=1 scripts/check_kani_advanced.sh
```

To prove the three bounded secret-frame properties added for the `2.0`
development API, use:

```sh
BASE64_NG_KANI_PROVE_SECRET_FRAMES=1 scripts/check_kani_advanced.sh
```

These proofs establish that a stack-backed frame never releases more than its
declared decoded bound, oversized public input is rejected before the
fixed-work scanner runs, and borrowed public output is written only after a
valid final gate. They are opt-in because the arbitrary-input release-gate
proof expands the fixed 64-entry scans and volatile cleanup loops into a much
larger SAT instance than the mandatory 28-harness baseline.

To prove the three bounded Commit 20 secret-encoder properties, use:

```sh
BASE64_NG_KANI_PROVE_SECRET_ENCODING=1 scripts/check_kani_advanced.sh
```

These proofs cover final-quantum output bounds, absorbing oversized-input
failure, and overlap/address-range preflight. Whole-frame cleanup remains in
deterministic drop/unwind tests and Miri because Kani's model of volatile wipe
and compiler-barrier paths makes symbolic whole-frame proofs impractical.

To run the broad public strict-decode no-panic proof, use:

```sh
BASE64_NG_KANI_PROVE_PUBLIC_SURFACE=1 scripts/check_kani_advanced.sh
```

The symbolic wrapped-decode harnesses are classified as exploratory because
they can consume very large amounts of memory and run for hours. To run them
manually, use:

```sh
BASE64_NG_KANI_EXPENSIVE_WRAPPED=1 scripts/check_kani_advanced.sh
```

These harnesses are not release proof. Deterministic, property, fuzz, and Miri
evidence covers wrapped public surfaces instead.

## Harness Scope

Current harnesses cover:

- scalar `decode_chunk` output bounds and bit-packing agreement with decoded
  6-bit values
- unpadded scalar tail validation and decode output bounds
- scalar length-helper bounds
- validated-alphabet per-position constructor bounds for an arbitrary 64-byte
  table and arbitrary in-range constructor position, including diagnostic
  indexes and duplicate-scan cursor bounds
- bounded scalar encode/decode output-prefix bounds
- strict decode backend agreement with the scalar reference for one padded
  quantum
- in-place decode prefix bounds
- clear-tail cleanup behavior on decode failures
- constant-time-oriented validate/decode agreement for one quantum
- bounded ordinary-array visible-length construction
- exact three-byte const encoding against the selected alphabet table
- reverse in-place encode cursor ordering over every bounded length and both
  padding shapes
- forward in-place decode cursor ordering across bounded complete quanta and
  tails; the proof and production kernel call the same encoded-tail,
  decoded-quantum, and decoded-tail length helpers so their arithmetic cannot
  drift independently
- all strict 2.0 preset aliases, encoded-length formulas, runtime policy
  validation, incremental finalization, and overlap preflight
- portable SIMD block arithmetic, ASCII classification, all-lane masks,
  backend-width cursor bounds, and initialized-before-visible commits

The default advanced script checks code generation for:

- bounded secret-frame release, pre-scan oversize rejection, and borrowed
  output release-gate properties
- broad no-panic exercise of selected public strict decode surfaces for an
  8-byte symbolic input
- strict wrapped decode output-prefix bounds for an 8-byte symbolic input
- strict wrapped clear-tail cleanup for an 8-byte symbolic input

The `BASE64_NG_KANI_PROVE_PUBLIC_SURFACE=1` harness additionally proves:

- broad no-panic exercise of selected public strict decode surfaces for an
  8-byte symbolic input

The `BASE64_NG_KANI_PROVE_SECRET_FRAMES=1` harness set additionally proves:

- stack-backed decoded output length remains within its declared bound and its
  unused tail remains cleared
- oversized public input is rejected before scanning and leaves the state
  absorbing
- borrowed public output remains unavailable until the final validity gate,
  while both staging and public storage are cleared after release or rejection

The `BASE64_NG_KANI_PROVE_FINAL_CORE=1` harness set additionally proves:

- all four strict preset encoders refine a deliberately independent fixed-array
  RFC 4648 oracle for every bit of one quantum and every tail shape
- Standard and URL-safe production alphabet lookup plus production decode
  packing refine their independent position and bit formulas compositionally
- exhaustive and differential tests connect the proof components to arbitrary
  incremental and in-place inputs; two integrated Kani versions are retained
  as exploratory because production lookup expansion exceeds 16 GiB
- strict canonical trailing-bit rejection
- incremental and in-place decode refinement and complete-buffer rollback
- validated built-in alphabets are runtime-constructor fixed points

The `BASE64_NG_KANI_PROVE_ASSURANCE=1` harness set additionally proves the
bounded in-memory teardown and journal model: four-axis closure, exact pending
stage, retained wipe evidence, generation rejection, no replay, exactly-once
accounting, monotonic progress, and non-owning tombstones.

This model does not prove persistence, crash recovery, OS protection, allocator
behavior, or an external unsafe provider. The complete claim boundary is in
[`2.0_FORMAL_VERIFICATION.md`](2.0_FORMAL_VERIFICATION.md).

The `BASE64_NG_KANI_EXPENSIVE_WRAPPED=1` exploratory set attempts:

- strict wrapped decode output-prefix bounds for an 8-byte symbolic input
- strict wrapped clear-tail cleanup for an 8-byte symbolic input
- incremental and in-place RFC known answers across every tail length
- production in-place complete-buffer rollback after validation failure
- borrowed secret-frame release gating through the full fixed-work decoder

These six harnesses are not counted as release proof unless they complete
within a separately reviewed resource profile.

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
- reproducible commands that work with the documented Kani toolchain
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
cargo install --locked kani-verifier --version 0.67.0
cargo kani setup
cargo kani --version
scripts/check_kani.sh
```

For future releases, revisit this document whenever the installed Kani release,
the documented Kani toolchain, the active release toolchain, or the harness
unwind policy changes.
