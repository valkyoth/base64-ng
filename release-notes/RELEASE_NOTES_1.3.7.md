# base64-ng 1.3.7

`1.3.7` is a maintenance and crate-family version synchronization patch for
the `base64-ng` crate family.

## Highlights

- Synchronized all workspace crate package versions to `1.3.7`.
- Updated public dependency examples, migration guidance, crate-family release
  metadata, and publishing order for the new patch version.
- Updated the active release toolchain to Rust `1.97.0` while continuing to
  test Rust `1.90.0` as the MSRV.
- Added a fail-closed CI assertion that the selected `rustc` exactly matches
  the release-toolchain pin.
- Updated test-only byte iteration syntax for Rust `1.97.0` Clippy.
- Updated the constant-time assembly evidence parser for Rust `1.97.0`'s v0
  symbols while retaining compatibility with legacy Rust symbol mangling.
- Updated the optional bytes and sanitization companions to `bytes` `1.12.1`
  and exact-pinned `sanitization` `1.2.4`.
- Pinned GitHub Actions to checkout `v6.0.2`, rust-cache `v2.9.1`, and
  install-action `v2.83.0`, using their verified release commits.
- Added the 210-test nextest suite to GitHub CI and disabled install-action
  fallback installation.
- Pinned documented release/deep-check tool versions to the current audited
  releases, including cargo-nextest `0.9.140` and cargo-fuzz `0.13.2`.
- Scheduled the stronger RISC-V RVV proof and backend-admission review for
  `1.3.8`, preserving the current QEMU-tested scalar/fallback posture until a
  dedicated evidence cycle is complete.

## Notes

No core encode/decode logic, SIMD admission scope, unsafe boundary, target
evidence, or core zero-dependency policy changes in this release. Existing RISC-V QEMU
evidence remains valid functional and fallback evidence, but it is not real
hardware acceleration, performance, timing, side-channel, or
register-retention proof.
