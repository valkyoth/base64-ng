# base64-ng 1.3.5

`1.3.5` is a RISC-V evidence and release-governance patch for the
`base64-ng` crate family.

## Highlights

- Added required `riscv64gc-unknown-linux-gnu` QEMU user-mode evidence for
  functional correctness and scalar/fallback runtime behavior.
- Added release-gated RISC-V posture checks documenting that stable Rust does
  not currently expose an admitted RVV backend for `base64-ng`.
- Documented that RISC-V acceleration is deliberately not admitted in this
  release. QEMU evidence is correctness/fallback evidence only, not hardware
  performance, timing, side-channel, register-retention, or production-CPU
  equivalence evidence.
- Added RISC-V release evidence into the stable release gate alongside the
  existing x86, AArch64, wasm, and big-endian evidence paths.
- Synchronized all workspace crate package versions to `1.3.5`.

## Notes

The admitted SIMD acceleration scope is unchanged from `1.3.4`. RISC-V targets
remain scalar/fallback-only until stable Rust exposes reviewed RVV intrinsic
paths and the project receives real hardware evidence from capable RVV systems.
