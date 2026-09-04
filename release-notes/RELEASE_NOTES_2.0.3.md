# base64-ng 2.0.3

`base64-ng` 2.0.3 is a synchronized maintenance release. The core runtime
implementation and public 2.0 API are unchanged.

## Changes

- Updates the active release toolchain to Rust 1.98.1 while retaining verified
  compatibility with the Rust 1.90.0 MSRV.
- Updates the exact-pinned `sanitization` companion dependency to 2.0.4.
- Updates `taiki-e/install-action` to 2.87.4 with an immutable commit pin.
- Confirms the remaining Rust dependencies, release tools, and GitHub Actions
  are current at release initiation.
- Synchronizes all Rust companion crates and
  `@valkyoth/base64-ng-wasm-loader` at 2.0.3.
- Makes high-assurance target eligibility independent of additive SIMD feature
  unification, with compile-fail regression checks for wasm and unattested
  AArch64.

No codec behavior, public API contract, SIMD admission decision, or MSRV
change is included.
