# base64-ng 2.0.1

`base64-ng` 2.0.1 is a packaging and documentation maintenance release. The
runtime implementation and public 2.0 API are unchanged from 2.0.0.

## Changes

- Reduces the published core archive by excluding repository-only release,
  evidence, API-snapshot, portability, and engineering-process files.
- Keeps the complete source, tests, licenses, changelog, security policy, and
  user-facing README in the crates.io package.
- Replaces the oversized root README with a concise user guide and links to
  the full engineering and assurance documentation on GitHub.
- Synchronizes all Rust companion crates and
  `@valkyoth/base64-ng-wasm-loader` at 2.0.1.

No codec behavior, API contract, SIMD admission decision, secret-processing
boundary, dependency, or MSRV changes in this release.
