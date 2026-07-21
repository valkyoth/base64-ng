# base64-ng 1.3.9

`1.3.9` migrates the optional sanitization companion to the security-boundary
changes in `sanitization` `2.0.3` and keeps the workspace crate family version
aligned.

## Highlights

- Synchronized all workspace crate versions to `1.3.9`.
- Exact-pinned `base64-ng-sanitization` to `sanitization` `2.0.3`.
- Preserved the existing fixed locked decode return type and added a separate
  fill-error method for sanitization 2.0 integrity-aware initialization.
- Added a direct return-type regression test because trait-method signature
  changes are not reliably detected by ordinary semver tooling.
- Added fallible integrity-checked comparison helpers for locked fixed and
  dynamic containers.
- Made checked fixed-size locked decode establish required memory-lock, dump,
  and fork controls before plaintext materialization.
- Made built-in checked dynamic decode use the protected-capacity constructor,
  which establishes required controls before invoking the decode closure.
- Rejects malformed or wrong-length fixed input before protected mapping
  allocation and checks the final decoded length in release builds before
  initializing locked storage.
- Adds counter-backed allocation-boundary tests proving rejected fixed and
  dynamic inputs cannot invoke protected allocation.
- Added deterministic admission tests proving degraded dynamic containers are
  dropped instead of returned.
- Updated the active release toolchain to Rust `1.97.1`, Serde to `1.0.229`,
  Tokio to `1.53.1`, and immutable GitHub Action pins to current releases.
- Updated locked-container examples to use checked exposure APIs.
- Strengthened `high-assurance` with strict random canaries and strict assembly
  comparison.
- Added the new `strict-compare` feature name while retaining `strict-ct` as a
  migration alias.
- Moved the RISC-V RVV proof and backend-admission review to `1.3.10`.

## Compatibility

The core `base64-ng` API and encoded/decoded output are unchanged. The existing
`base64-ng-sanitization` fixed locked decode method retains its generation
error type; callers can opt into the additive `_fill` method for sanitization
2.0's fill-error model. Existing non-locked comparison helpers remain
unchanged. Existing locked comparison helpers remain available as a fail-stop
compatibility path, while new code should prefer
`LockedSanitizationCtEqExt` to propagate canary-integrity failures.
High-assurance fixed-size deployments should use
`decode_locked_secret_bytes_checked`. Built-in dynamic callers can use
`decode_locked_secret_vec_checked` for the same pre-decode required-control
boundary. External extension-trait implementations must override the
compatibility default to make that guarantee.

No SIMD backend, unsafe boundary, or core runtime dependency is added by this
release.
