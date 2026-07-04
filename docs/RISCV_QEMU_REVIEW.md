# RISC-V QEMU Review

This review tracks the `1.3.5` RISC-V evidence line.

## Status

RISC-V is currently a **QEMU-tested scalar/fallback target**, not an admitted
accelerated backend. The project-owned evidence runs
`riscv64gc-unknown-linux-gnu` under `qemu-riscv64` and checks that the public
encode, strict decode, in-place, clear-tail, wrapped/legacy, streaming, and
runtime-report surfaces continue to match scalar behavior.

The required path is:

```text
scripts/check_riscv_qemu.sh
```

## What QEMU Evidence Covers

QEMU evidence is accepted for:

- functional encode/decode correctness under the Rust `riscv64gc` target;
- malformed-input and public error-shape behavior;
- caller-owned buffer and clear-tail behavior;
- in-place and staged surface behavior;
- stream adapter behavior;
- runtime reporting that remains scalar active on RISC-V today;
- release reproducibility for the cross-target command path.

QEMU evidence is not accepted for:

- real RVV hardware performance claims;
- timing or microarchitectural behavior;
- side-channel claims;
- register-retention cleanup evidence;
- claims that QEMU behavior is identical to any production RISC-V core.

## Stable Rust Blocker

On the active release toolchain, `core::arch::riscv64` remains behind the
unstable `riscv_ext_intrinsics` feature gate. The release gate records this
with:

```text
scripts/check_riscv_intrinsics_status.sh
```

Because stable Rust does not yet provide a reviewed RVV intrinsic surface for
this crate, `1.3.5` does not add an RVV backend and does not describe RISC-V as
accelerated.

## Admission Rule

RISC-V runtime reports must remain scalar active until a future release links
real hardware evidence. Any future RVV encode or strict decode backend must be
QEMU-tested until real RVV hardware evidence is linked.

Required before upgrading from QEMU-tested scalar/fallback to
hardware-attested RVV acceleration:

- exact board/SoC and vector width, for example a Vector 1.0 SpacemiT K1/X60
  class system;
- kernel, userspace, QEMU/native runner, Rust target, and compiler versions;
- generated assembly review for each admitted backend;
- unsafe-boundary and register-cleanup review;
- scalar differential tests for encode, strict decode, tails, padding,
  malformed input, clear-tail, and staged in-place surfaces;
- benchmarks on real hardware;
- side-channel caveat review for the deployment profile.

Until those artifacts exist, release notes and docs must state that RISC-V
evidence is QEMU functional evidence only.
