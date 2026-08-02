# Commit 33 AArch64 SVE Review

Commit 33 adds a complete, non-admitted SVE candidate for ordinary Standard
and URL-safe encode and strict decode. The implementation ships in the 2.0
source, but normal published builds continue to use admitted NEON or scalar.
The candidate is compiled only when project-owned evidence sets the internal
`base64_ng_sve_candidate` cfg.

## Current Decision

QEMU proves candidate functional behavior at vector lengths of 128, 256, and 512 bits. It does not prove real silicon behavior, performance, timing, ABI or
signal-state preservation, context switching, or deployment safety. Safe
automatic SVE dispatch therefore remains disabled. Admission requires accepted
reports from two real SVE systems with different vector lengths.

Inside the evidence build, runtime reporting shows `candidate = sve` while
ordinary encode and strict decode continue through admitted NEON. Secret
encode/decode remains the separate scalar fixed-work boundary.

## Candidate Algorithm

The isolated base-PCS leaf functions use SVE integer operations:

- encode activates four byte lanes, loads four 3-byte groups with `ld3b`,
  extracts four 6-bit vectors, maps Standard or URL-safe ASCII with predicates
  and `sel`, and stores four interleaved vectors with `st4b`;
- strict decode first completes scalar validation, loads four ASCII vectors
  with `ld4b`, maps validated characters, reconstructs three byte vectors, and
  stores with `st3b`;
- padding, short tails, unsupported/custom alphabets, and detailed error
  reporting remain scalar;
- `ptrue ... vl4` makes the fixed-block algorithm independent of the current
  physical vector length;
- only caller-saved `z0..z7` and `p0..p1` are touched, and all are cleared
  before return.

Stable Rust 1.97.1 recognizes `+sve` but does not provide the stable
per-function intrinsic boundary required here. Commit 33 therefore uses four
leaf `global_asm!` functions with no calls or stack mutation. Normal artifacts
do not compile this module.

## Runtime Detection

Linux and Android `std` evidence checks capability on the current thread for
every candidate call:

1. require the `HWCAP_SVE` bit from `AT_HWCAP`;
2. call `PR_SVE_GET_VL` and require a valid 16-byte multiple in the architectural
   16..=256-byte range;
3. reject missing capability, failed `prctl`, malformed lengths, unknown
   operating systems, and big-endian AArch64;
4. re-query rather than cache because SVE vector length is thread-local and may
   change during process execution.

The no-std evidence build requires compile-time `+sve`; it does not perform
runtime probing.

## QEMU Evidence

Run:

```text
scripts/check_sve_qemu.sh
scripts/generate_sve_asm_evidence.sh
```

The QEMU gate runs complete default, all-feature, no-default, and doctest
suites on statically linked `aarch64-unknown-linux-musl`. It then runs candidate
differential encode/decode, malformed-input transactionality, runtime-report,
probe, and per-thread vector-length-change tests at 128, 256, and 512 bits.

The assembly gate requires all five leaf symbols, SVE structured loads/stores,
predicate mapping, vector-length discovery, caller-saved register cleanup, no
nested calls, and no stack mutation.

## Real Hardware Admission

Real reports must follow `hardware-evidence/sve/schema-v1.json` and be produced
from a clean exact commit with `scripts/check_sve_hardware.sh`. Each report
records the exact system, CPU, firmware, kernel, compiler, vector length,
HWCAP/`PR_SVE_GET_VL` state, native tests, benchmarks, ABI/signal review,
assembly, cleanup, and pentest range.

One report cannot admit the backend. A later reviewed dispatch commit must
consume reports from at least two real SVE systems with different vector
lengths, establish beneficial thresholds against NEON, and integrate SVE into
operation health quarantine and fallback. Candidate reports require
`production_admitted = false`.

## Residual Constraints

- This is ordinary-data acceleration, not a constant-time or secret path.
- QEMU cannot establish hardware performance, register remanence, speculative
  behavior, cache/timing properties, signal delivery, or context switching.
- The candidate rechecks thread vector length before each slice operation, but
  native review must still cover external vector-length changes and FFI calls.
- Stable intrinsic support must be re-evaluated before admitting an assembly
  implementation to automatic dispatch.
