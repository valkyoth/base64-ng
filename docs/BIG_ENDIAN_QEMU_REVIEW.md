# Big-Endian QEMU Review

This review tracks the `1.3.4` big-endian evidence line.

## Status

Big-endian targets are currently **QEMU-tested scalar/fallback targets**, not
admitted accelerated backends.

The local evidence path is:

```sh
scripts/check_big_endian_qemu.sh
```

The required path is `s390x-unknown-linux-gnu` through `qemu-s390x`. The
optional path is `powerpc64-unknown-linux-gnu` through `qemu-ppc64` when a
complete local PowerPC64 glibc sysroot is available.

## Evidence Boundary

QEMU evidence is accepted for:

- functional encode/decode correctness
- malformed-input behavior
- clear-tail behavior
- in-place behavior
- wrapped and legacy compatibility behavior
- stream behavior
- scalar/fallback runtime reporting

QEMU evidence is not accepted for:

- real hardware performance claims
- timing or side-channel claims
- microarchitectural behavior
- register-retention behavior on production silicon
- proof that a particular production CPU executes the same path

## Stable Rust Toolchain Blocker

The pinned release toolchain is Rust `1.90.0`. On that toolchain:

- `core::arch::s390x` is gated by the unstable `stdarch_s390x` feature.
- `core::arch::powerpc64` is gated by the unstable `stdarch_powerpc` feature.

Those gates prevent a stable, no-dependency Rust implementation from using the
normal intrinsic-based vector path for s390x or PowerPC64 today.

Hand-written inline assembly is not accepted as a shortcut for this release
line. It would require a separate unsafe-boundary review, generated assembly
review, register cleanup review, fallback evidence, and real hardware reports
before it could be described as hardware-attested acceleration.

## Admission Rule

Until stable Rust exposes a reviewed intrinsic path, or the project accepts a
separate assembly-backed backend review, big-endian runtime reports must remain scalar active.
Any future s390x or PowerPC64 acceleration must be labeled as QEMU-tested until real hardware evidence is linked.

Required before upgrading from QEMU-tested to hardware-attested:

- exact hardware model and CPU feature report
- kernel/runtime version
- Rust toolchain and target triple
- generated assembly evidence
- scalar differential tests for encode and strict decode
- malformed-input, padding, tail, and clear-tail evidence
- register cleanup review
- benchmark data
- pentest review for the exact commit range
