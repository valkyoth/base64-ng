# Big-Endian QEMU And Hardware Review

Status: Commit 31 scalar/fallback evidence complete under QEMU; real hardware
evidence pending.

## Current Posture

Big-endian targets are **QEMU-tested scalar/fallback targets**, not admitted
accelerated backends. The release gate requires complete emulated suites for:

- `s390x-unknown-linux-gnu` through `qemu-s390x`;
- `powerpc64-unknown-linux-gnu` through `qemu-ppc64`.

Run both required targets with:

```sh
scripts/check_big_endian_qemu.sh --all
```

`--s390x` and `--powerpc64` are diagnostic modes. A single-target result is
not complete Commit 31 or release evidence.

## Functional Coverage

Each target receives default, all-feature, and no-default-feature test suites,
all-feature and no-default-feature doctests, and a no-std build with secret and
SIMD feature boundaries enabled. This includes:

- RFC 4648 vectors, strict padding and trailing-bit behavior;
- malformed input, transactional output, and clear-tail behavior;
- incremental, stream, wrapped, legacy-whitespace, and in-place surfaces;
- fixed and allocated secret storage, rejection cleanup, and assurance tests;
- scalar fallback, backend health, and operation-specific runtime reporting;
- the dedicated big-endian profile and in-place regression suite.

The byte-order reasoning and mechanically enforced source boundaries are in
[`2.0_BIG_ENDIAN_AUDIT.md`](2.0_BIG_ENDIAN_AUDIT.md).

## Evidence Boundary

QEMU evidence is accepted for functional correctness, target compilation, and
scalar/fallback reporting. QEMU evidence is not accepted for:

- real hardware performance claims;
- timing or side-channel claims;
- microarchitectural or register-retention behavior;
- physical cleanup behavior;
- proof that a production CPU executes identically to QEMU.

## Stable Rust Acceleration Blocker

On the active Rust `1.97.1` release toolchain:

- `core::arch::s390x` remains gated by `stdarch_s390x`;
- `core::arch::powerpc64` remains gated by `stdarch_powerpc`.

`scripts/check_big_endian_intrinsics_status.sh` fails if this changes so the
implementation and admission decision must be revisited. Hand-written inline
assembly is not accepted as a shortcut. It would require a separate unsafe and
ABI review, generated assembly, unwind and register-cleanup evidence, fallback
tests, and real-hardware correctness and performance results.

Therefore big-endian runtime reports must remain scalar active. No s390x,
PowerPC64, or big-endian AArch64 acceleration is admitted by Commit 31.

## Community Hardware Contract

Real-hardware operators should run:

```sh
scripts/check_big_endian_hardware.sh
```

The machine must report big-endian compilation, use a clean exact commit, and
pass the complete native test and documentation suites. Reports use
`hardware-evidence/big-endian/schema-v1.json` and are checked with:

```sh
scripts/validate-big-endian-hardware-evidence.py REPORT.json
```

The schema requires hardware, firmware, OS, kernel, compiler, exact commit,
transcript hash, scalar backend report, and pentest provenance. Validation is
structural, not authentication. Maintainers must review raw output and
provenance. Reports remain QEMU-tested until real hardware evidence is linked.

Any future acceleration additionally requires direct differential and
malformed-input tests, padding and tail coverage, assembly and ABI review,
register cleanup, backend quarantine and fallback evidence, representative
benchmarks, and external endian/alignment review for the exact commit.
