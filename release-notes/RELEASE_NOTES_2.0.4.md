# base64-ng 2.0.4

`base64-ng` 2.0.4 is a selective maintenance release. The core runtime
implementation, public 2.0 API, MSRV, and SIMD admission decisions are
unchanged.

## Changes

- Updates the exact-pinned `sanitization` companion dependency to 2.1.0 and
  verifies its complete supported feature matrix.
- Updates `taiki-e/install-action` to 2.87.7 with an immutable commit pin.
- Pins `Swatinem/rust-cache` 2.9.2 to the signed release tag's target commit.
- Confirms the remaining Rust dependencies, release tools, and GitHub Actions
  are current at release initiation.
- Retains Rust 1.98.1 for active release checks and Rust 1.90.0 as MSRV.
- Publishes only `base64-ng` and `base64-ng-sanitization` at 2.0.4. The eleven
  unchanged Rust companions and `@valkyoth/base64-ng-wasm-loader` remain at
  2.0.3 and continue to accept the compatible core patch.
- Hardens selective release validation against downgrade or major/minor drift
  and gives the independently versioned npm loader an explicit publish entry.
