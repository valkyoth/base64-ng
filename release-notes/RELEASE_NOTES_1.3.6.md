# base64-ng 1.3.6

`1.3.6` is a documentation and crate-family version synchronization patch for
the `base64-ng` crate family.

## Highlights

- Synchronized all workspace crate package versions to `1.3.6`.
- Added consistent companion-crate README headers with the shared project
  image, core documentation links, and crate-specific one-line summaries.
- Updated public examples and migration snippets to reference the `1.3.6`
  crate family.
- Moved the top constant-time guide example to
  `decode_slice_staged_clear_tail` so shared-memory and HSM-adjacent users see
  the staged decode pattern first.
- Added the custom `base64_ng_require_high_assurance` cfg. When set, builds
  that also enable `simd` fail at compile time.

## Notes

No encode/decode logic, SIMD admission scope, target evidence posture, unsafe
boundary, or runtime dependency policy changes in this release. The
high-assurance guard is a custom cfg, not a Cargo feature, so normal
`--all-features` release evidence and docs.rs builds remain usable.
