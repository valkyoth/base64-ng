# base64-ng 1.3.9

`1.3.9` migrates the optional sanitization companion to the security-boundary
changes in `sanitization` `2.0.1` and keeps the workspace crate family version
aligned.

## Highlights

- Synchronized all workspace crate versions to `1.3.9`.
- Exact-pinned `base64-ng-sanitization` to `sanitization` `2.0.1`.
- Updated fixed locked decode to return sanitization 2.0's fill-error type.
- Added fallible integrity-checked comparison helpers for locked fixed and
  dynamic containers.
- Updated locked-container examples to use checked exposure APIs.
- Strengthened `high-assurance` with strict random canaries and strict assembly
  comparison.
- Added the new `strict-compare` feature name while retaining `strict-ct` as a
  migration alias.
- Moved the RISC-V RVV proof and backend-admission review to `1.3.10`.

## Compatibility

The core `base64-ng` API and encoded/decoded output are unchanged. The
`base64-ng-sanitization` fixed locked decode error type now follows
`sanitization` 2.0's `LockedSecretBytesFillError`; callers matching the old
generation error must update those matches. Existing non-locked comparison
helpers remain unchanged. Existing locked comparison helpers remain available
as a fail-stop compatibility path, while new code should prefer
`LockedSanitizationCtEqExt` to propagate canary-integrity failures.

No SIMD backend, unsafe boundary, or core runtime dependency is added by this
release.
