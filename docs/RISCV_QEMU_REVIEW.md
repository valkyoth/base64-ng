# Commit 32 RISC-V Vector Review

Commit 32 adds a complete, non-admitted RVV 1.0 candidate for ordinary
Standard and URL-safe encode and strict decode. Accordingly, normal published builds remain scalar
on RISC-V. The candidate is compiled only when
project-owned evidence sets the internal `base64_ng_rvv_candidate` cfg.

## Current Decision

QEMU proves that the candidate computes the right bytes. It does not prove
real silicon behavior, ABI preservation, performance, timing, or deployment
safety. Production dispatch therefore remains scalar; real hardware evidence remains mandatory before RVV can be admitted.

The candidate is deliberately visible in runtime reporting only inside the
internal evidence build:

- `candidate = rvv`;
- candidate detection mode is runtime CPU features under Linux `std`;
- active encode and strict-decode backends remain `scalar`;
- ordinary acceleration remains false;
- secret encode/decode remains the separate scalar fixed-work boundary.

## Candidate Algorithm

The isolated leaf functions use RVV 1.0 basic integer operations:

- encode loads four 3-byte groups with `vlseg3e8.v`, extracts four 6-bit
  vectors, maps Standard or URL-safe ASCII with masks/arithmetic, and stores
  four interleaved vectors with `vsseg4e8.v`;
- strict decode first performs complete scalar validation, then loads four
  ASCII vectors with `vlseg4e8.v`, maps validated characters, reconstructs
  three byte vectors, and stores with `vsseg3e8.v`;
- padding, short tails, unsupported/custom alphabets, and all error reporting
  remain scalar;
- each leaf sets an explicit four-lane `e8,m1` vector length, so the algorithm
  is independent of the physical VLEN;
- every used register from `v0` through `v15` is cleared at VLMAX before
  return.

Stable Rust 1.97.1 rejects both RVV intrinsics and per-function
`#[target_feature(enable = "v")]`. Commit 32 consequently uses four leaf
`global_asm!` functions with no calls or stack mutation. The candidate ELF is
marked `rv64gcv`; normal artifacts do not compile this module.

## Runtime Detection

Linux `std` evidence uses a minimal reviewed UAPI boundary:

1. query `riscv_hwprobe` key `RISCV_HWPROBE_KEY_IMA_EXT_0` for the `V` bit;
2. query `PR_RISCV_V_GET_CONTROL` and require current vector state `ON`;
3. when an old kernel or QEMU does not implement those calls, use the startup
   `AT_HWCAP` `V` bit as the fail-closed fallback;
4. reject contradictory, disabled, missing, or malformed results.

The complete probe is repeated at every candidate entry. In particular,
`PR_RISCV_V_GET_CONTROL` is never process-cached because Linux defines vector
control for the calling thread; a result from one thread cannot authorize RVV
execution on another.

Pure parsing tests cover successful probes, old-kernel fallback, disabled
vector state, missing `V`, and contradictory results. Non-Linux `std` builds
return unavailable. `no_std` evidence requires compile-time `+v`; it never
performs runtime probing.

## QEMU Evidence

Run:

```text
scripts/check_riscv_qemu.sh
```

The gate runs complete default, all-feature, no-default, and doctest suites on
`riscv64gc-unknown-linux-gnu`. It then compiles the internal candidate and runs
its encode/decode differential, malformed-input, detection, and scalar-public-
dispatch tests at VLEN 128 and VLEN 256. It also checks the `no_std` static
`+v` build.

Generated assembly is checked by:

```text
scripts/generate_rvv_asm_evidence.sh
```

That gate requires all five leaf symbols, segmented vector loads/stores,
mask-based mapping, VLMAX cleanup, no nested calls, and an ELF `V` attribute.

## Real Hardware Admission

Real reports must follow
`hardware-evidence/riscv/schema-v1.json` and be generated from a clean exact
commit with `scripts/check_riscv_hardware.sh`. The report rejects QEMU and
virtual machines and requires:

- exact board, SoC, CPU, firmware, kernel, Rust, and Cargo versions;
- RVV 1.0, measured VLEN, `hwprobe` `V`, and enabled per-thread vector state;
- signal/context-switch and FFI ABI review;
- native differential tests and generated assembly review;
- raw benchmark data proving encode and decode benefit;
- register-cleanup review and a pentest range ending at the source commit.

An accepted Commit 32 report still says `production_admitted = false`. A later
reviewed commit must consume the evidence, set measured thresholds, integrate
health quarantine, and deliberately add RVV to production encode/decode
dispatch. QEMU evidence alone can never make that change.

## Residual Constraints

- The assembly boundary is ordinary-data code and makes no constant-time or
  secret-processing claim.
- QEMU does not establish register remanence, speculative execution, cache,
  timing, signal delivery, context-switch, or performance behavior.
- Thread migration and vector-state changes after detection remain part of the
  native admission review.
- Stable Rust intrinsic support must be re-evaluated before carrying assembly
  into the final 2.0 admission matrix.
