# RISC-V Vector Exact-Profile Review

Commit 32 introduced the complete RVV 1.0 candidate. The Commit 54 pre-seal
amendment adds an exact-profile RVV 1.0 backend for ordinary Standard and
URL-safe encode and strict decode on Linux `riscv64` with a measured SpacemiT
X60. Every other RISC-V profile remains scalar.
In short, other RISC-V profiles remain scalar and make no acceleration claim.

## Current Decision

QEMU proves that the leaf implementation computes the right bytes across VLEN
128 and VLEN 256 and that non-vector artifacts retain scalar fallback. QEMU
does not prove real-silicon behavior, ABI preservation, performance, timing,
or deployment safety. Those claims come only from the retained physical
Banana Pi BPI-F3 / SpacemiT X60 campaign.

Normal runtime reporting uses these rules:

- the exact X60 profile reports `candidate = rvv`;
- qualifying Standard/URL-safe calls report active `rvv` after their KAT;
- unknown hardware, failed probes, non-Linux, `no_std`, short inputs, and
  unsupported alphabets report or execute `scalar`;
- QEMU candidate builds may report `candidate = rvv` while public dispatch
  remains `scalar` because QEMU is not the admitted identity;
- secret encode/decode remains the separate scalar fixed-work boundary.

## Candidate Algorithm

The isolated leaf functions use RVV 1.0 basic integer operations:

- encode batches as many complete 3-byte groups as the active VLEN permits
  with `vlseg3e8.v`, extracts four 6-bit vectors, maps Standard or URL-safe
  ASCII with masks/arithmetic, and stores four interleaved vectors with
  `vsseg4e8.v`;
- strict decode first performs complete scalar validation, then loads four
  batched ASCII vectors with `vlseg4e8.v`, maps validated characters,
  reconstructs three byte vectors, and stores with `vsseg3e8.v`;
- padding, short tails, unsupported/custom alphabets, and all error reporting
  remain scalar;
- each leaf loops with `vsetvli e8,m1`, so it uses the physical VLEN without
  embedding a VLEN-specific load, store, or pointer stride;
- one leaf call processes every complete input quantum and clears vector
  registers once after the batch rather than once per small fixed block;
- every used register from `v0` through `v15` is cleared at VLMAX before
  return.

Stable Rust 1.97.1 rejects both RVV intrinsics and per-function
`#[target_feature(enable = "v")]`. Commit 32 consequently uses four leaf
`global_asm!` functions with no calls or stack mutation. `.option arch,+a,+v`
scopes Atomic and Vector instruction acceptance to those leaves. The linked
artifact deliberately does not publish a global `V` ELF requirement, so it can
start and select scalar on RISC-V systems without Vector support.

## Runtime Detection

Linux `std` production admission uses a minimal reviewed UAPI boundary:

1. query `riscv_hwprobe` for `mvendorid`, `marchid`, `mimpid`, and
   `RISCV_HWPROBE_KEY_IMA_EXT_0`;
2. require the exact accepted identity `0x710`, `0x8000000058000001`, and
   `0x1000000049772200`, plus the RVV 1.0 `V` bit;
3. query `PR_RISCV_V_GET_CONTROL` and require current vector state `ON`;
4. reject old kernels, unavailable keys, fallback-only auxiliary-vector
   results, contradictory values, disabled state, and every other identity.

The internal QEMU candidate detector remains separate. It may use `AT_HWCAP`
when QEMU lacks `riscv_hwprobe`, but that result can never authorize production
dispatch.

The complete probe is cached independently on each calling thread. Linux does
not permit a thread to turn Vector off after it has been enabled, so a cached
positive remains valid until `execve`; a cached negative remains a conservative
scalar fallback if an application enables Vector later. A result from one
thread never authorizes RVV execution on another, and the crate does not call
`PR_RISCV_V_SET_CONTROL` or override parent-process policy.

Pure tests reject every independently corrupted identity, key, feature, and
thread-state field. Non-Linux `std` and all safe `no_std` builds return
unavailable. The internal `no_std +v` build remains compile/codegen evidence,
not production admission.

## QEMU Evidence

Run:

```text
scripts/check_riscv_qemu.sh
```

The gate runs complete default, all-feature, no-default, and doctest suites on
`riscv64gc-unknown-linux-gnu` with vector execution explicitly disabled. It
then compiles the internal candidate and runs its encode/decode differential,
malformed-input, detection, and scalar-public-dispatch tests with RVV 1.0 at
VLEN 128 and VLEN 256. Every QEMU-hosted Rust harness is serialized because
Ubuntu 24.04 currently ships QEMU 8.2, whose user-mode RVV implementation can
crash internally while a parallel test harness creates threads. Serialization
is an emulator compatibility constraint, not evidence of thread safety or real
hardware behavior. The gate also checks the `no_std` static `+v` build.

Generated assembly is checked by:

```text
scripts/generate_rvv_asm_evidence.sh
```

That gate requires all seven evidence symbols, segmented vector loads/stores,
mask-based mapping, VLMAX cleanup, no nested calls, and verifies that RVV stays
leaf-local instead of becoming a global ELF `V` requirement.

## Real Hardware Admission

Physical evidence was captured on a Banana Pi BPI-F3 with a SpacemiT X60,
RVV 1.0, and VLEN 256. The clean pre-integration campaign passed correctness,
malformed rejection, Linux signal-frame restoration, thread context switches,
FFI ABI checks, register cleanup, generated assembly review, and the retained
15-sample performance policy. That evidence supports only the named profile.
After dispatch integration, the same native gate must be rerun against the
exact final source commit before the 2.0 release seal.

The frozen host identity is Linux `riscv64`, `mvendorid=0x710`,
`marchid=0x8000000058000001`, and `mimpid=0x1000000049772200`. Evidence from
that host can support only that exact profile; it cannot establish behavior or
performance for other RVV implementations.

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

The resumable distributed evidence manager exposes this campaign as the
separate `riscv_hardware` job. It reuses an installed project Rust toolchain,
does not install nightly or `cargo-fuzz`, and retrieves the validated hardware
bundle separately from the 18 fuzz shards. The direct capture command is:

```text
scripts/capture-2.0-riscv-admission.sh \
  target/release-evidence/riscv-native-admission
```

The Linux signal-frame test is explicitly ignored in ordinary and QEMU suites
and run by exact name only from the native hardware gate. This prevents QEMU
signal emulation from being mistaken for kernel/hardware vector-state evidence.

Production code uses a 192-byte encode and strict-decode crossover, independent
operation KATs, process quarantine, per-thread capability detection, and scalar
fallback. The final retained report must name the integrated commit and set the
exact Linux/X60 admission scope. QEMU evidence alone can never satisfy it.

## Residual Constraints

- The assembly boundary is ordinary-data code and makes no constant-time or
  secret-processing claim.
- The admission does not extend to custom alphabets, CT secret paths,
  non-Linux systems, safe `no_std`, or any RISC-V identity other than the
  measured SpacemiT X60 values.
- QEMU does not establish register remanence, speculative execution, cache,
  timing, signal delivery, context-switch, or performance behavior.
- A second RVV implementation would require its own identity, native evidence,
  threshold, and reviewed admission change.
- Stable Rust intrinsic support should be re-evaluated during maintenance, but
  it does not widen the frozen exact-profile claim automatically.
