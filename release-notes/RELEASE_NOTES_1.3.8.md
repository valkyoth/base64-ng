# base64-ng 1.3.8

`1.3.8` is a security-hardening and crate-family synchronization patch for
the `base64-ng` workspace.

## Highlights

- Synchronized all workspace crate package versions to `1.3.8`.
- Hardened `base64-ng-tokio` read-all helpers so vector growth copies into a
  guarded replacement and wipes the old allocation before deallocation.
- Applied guarded growth to bounded and unbounded helpers, with regression
  coverage above the 8 KiB eager capacity.
- Hardened Chromium-family wasm runtime evidence so success requires a
  runtime-created DOM attribute that is absent from the static HTML source.
- Added a runtime alphabet-contract check that rejects hand-written
  `Alphabet::encode` overrides when they disagree with `Alphabet::ENCODE`.
- Added daily and manually dispatchable RustSec and cargo-deny monitoring for
  the workspace and isolated tool lockfiles.
- Updated async cleanup, release evidence, migration, and dependency examples.
- Moved the stronger RISC-V RVV proof and backend-admission review to `1.3.9`.

## Notes

This patch does not change core Base64 encode/decode behavior, SIMD admission,
unsafe boundaries, or the zero-runtime-dependency posture of `base64-ng`.
