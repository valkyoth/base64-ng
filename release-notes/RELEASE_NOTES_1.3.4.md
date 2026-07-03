# base64-ng 1.3.4

`1.3.4` is a big-endian evidence and release-governance patch for the
`base64-ng` crate family.

## Highlights

- Added required `s390x-unknown-linux-gnu` QEMU user-mode evidence for
  big-endian functional correctness and scalar/fallback runtime behavior.
- Added release-gated checks proving stable Rust still gates `s390x` and
  PowerPC64 vector intrinsics behind unstable `stdarch_*` features on the
  active release toolchain.
- Documented that big-endian acceleration is deliberately not admitted in this
  release. QEMU evidence is correctness/fallback evidence only, not hardware
  performance, timing, side-channel, register-retention, or production-CPU
  equivalence evidence.
- Improved the optional PowerPC64 QEMU path with an early glibc sysroot
  preflight, so maintainers get a clear missing-sysroot error when only the
  cross compiler is installed.
- Synchronized all workspace crate package versions to `1.3.4`.

## Notes

The admitted SIMD acceleration scope is unchanged from `1.3.3`. Big-endian
targets remain scalar/fallback-only until stable Rust exposes reviewed
intrinsic paths or the project accepts a separate assembly-backed backend
review with real hardware evidence.
