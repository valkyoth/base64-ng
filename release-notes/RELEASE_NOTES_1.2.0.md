# base64-ng 1.2.0 Release Notes

Status: released

## Summary

- Synced all workspace crates for the `1.2.0` family release after collecting
  the former `1.1.x` checkpoint work without publishing many small intermediate
  crates.io versions.
- Updated `base64-ng-sanitization` to `sanitization` `1.2.1`, added native
  `sanitization::ct::Choice` comparison helpers for decoded `SecretBytes` and
  `SecretVec` values, and added opt-in locked-secret decode helpers that write
  directly into `LockedSecretBytes` or `LockedSecretVec` under the companion's
  `memory-lock`/`high-assurance` feature set.
- Cached std SIMD backend detection after the first runtime probe, gated
  admitted encode dispatch by each backend's fixed block size, and avoided
  runtime SIMD wrapper calls for inputs that cannot fill a vector block.
- Removed extra per-block input/output stack copies from admitted runtime SIMD
  slice encode loops by passing bounds-proven fixed-size views directly into
  the reviewed block encoders.
- Clarified the `1.2.0` encode scope: slice, clear-tail, alloc, and wrapped
  encode helpers route through the encode backend boundary; wrapped encode may
  use SIMD for the unwrapped staging step, while line-ending insertion,
  in-place encode, tails, padding, custom alphabets, `no_std`, and decode stay
  scalar unless separately admitted.
- Admitted std `x86`/`x86_64` SSSE3/SSE4.1 encode dispatch for Standard and
  URL-safe alphabet families. The admitted path processes fixed 12-byte blocks
  with vector code after runtime CPU probing and falls back to scalar for
  unsupported CPUs, `no_std`, custom alphabets, tails, padding, in-place encode,
  line-ending insertion, and every decode path.
- Updated runtime reporting, backend evidence, SIMD admission validation,
  unsafe inventory, and user documentation so SSSE3/SSE4.1 encode is reported
  as an admitted backend while AVX2, AVX-512 VBMI, NEON, wasm `simd128`,
  custom-alphabet, in-place, and decode acceleration remain prototype-only or
  scalar.
- Added a real non-dispatchable AVX-512 VBMI fixed-block encode prototype that
  uses the provided alphabet table for all alphabets. The prototype remains
  test-only and is not reachable from runtime backend selection.
- Added AVX-512 SIMD equivalence coverage for patterned blocks, all 64 emitted
  six-bit Base64 values, and a non-standard custom alphabet.
- Added `scripts/generate_simd_asm_evidence.sh` to capture release
  test-harness assembly for inactive SSSE3/SSE4.1, AVX2, and AVX-512 VBMI
  encode prototypes.
- Added a real non-dispatchable AArch64 NEON fixed-block encode prototype for
  Standard and URL-safe alphabets. Custom alphabets and 32-bit `arm+neon`
  remain on scalar scaffold paths.
- Added NEON equivalence coverage for patterned blocks, all 64 emitted six-bit
  Base64 values, and custom alphabet fallback.
- Added a real non-dispatchable wasm `simd128` fixed-block encode prototype for
  Standard and URL-safe alphabets. Custom alphabets remain on the scalar
  scaffold path because portable wasm SIMD has no direct 64-byte lookup.
- Added wasm `simd128` test-binary compile evidence to the SIMD feature-bundle
  check while keeping wasm cleanup/JIT caveats and scalar runtime dispatch.
- Added `scripts/check_aarch64_linux.sh` for real ARM Linux host verification,
  including NEON encode block evidence, backend evidence, SIMD feature-bundle
  checks, and SIMD admission validators.
- Hardened SIMD admission tooling and backend evidence manifests to distinguish
  real non-dispatchable prototypes from admitted active backends.
- Added a draft SIMD encode admission package, including runtime-report
  expectations, benchmark record shape, and release-note wording rules for
  encode backend activation decisions.
- Added `scripts/validate-simd-encode-admission-draft.sh` and wired it into the
  standard checks so the encode-dispatch admission contract remains packaged
  and machine-checked.
- Wiped the small unpadded in-place decode tail buffer before return, expanded
  AArch64 NEON encode cleanup to all vector registers, and
  documented the wrapped slice encoder's temporary in-buffer staging behavior.
- Added `base64-ng-subtle` as an optional companion crate for projects that
  already admit `subtle` and want reviewed `ConstantTimeEq` comparisons for
  `base64-ng` buffers.
- Inlined the AArch64 NEON register cleanup macro into the encode block path so
  callee-saved `v8..v15` are not restored by a separate helper frame.
- Added a real non-dispatchable AVX2 fixed-block encode prototype for Standard
  and URL-safe alphabets. The prototype remains test-only and is not reachable
  from runtime backend selection.
- Added AVX2 SIMD equivalence coverage for patterned blocks, all 64 emitted
  six-bit Base64 values, and custom alphabets that must fall back to scalar
  encoding.
- Updated the SIMD unsafe inventory for the AVX-512, AVX2, and AArch64 NEON
  prototypes, including staged stack-copy wiping and vector-register cleanup.
- Refreshed the SIMD roadmap, admission manifest, and backend evidence output
  so `1.1.x` checkpoint tags consistently describe the current state as real
  non-dispatchable prototype evidence for backends that are not yet admitted.
- Hardened the workspace publish helper so real crates.io publishing requires
  `HEAD` to match a verified signed `v<version>` tag, and documented the
  `git tag -v` release check.
- Tightened high-assurance documentation after follow-up audit review: clarified
  conservative tail wiping, `DecodedBuffer` clone duplication, public-length
  `subtle` comparisons, strict decode error logging, AArch64/RISC-V deployment
  policy checks, and wrapped in-place decode retention behavior.
- Updated the roadmap to record `v1.1.3` and `v1.1.4` checkpoint scope and to
  spell out the remaining active encode-dispatch admission work before a
  future `1.2.0` family release.
- Expanded the SIMD roadmap so `1.2.0` explicitly means fully working encode
  acceleration for the admitted encode scope, with planned `1.1.5` through
  `1.1.12` checkpoints leading to that release.
- Added a scalar-forced encode backend boundary and routed public slice,
  clear-tail, wrapped, alloc, and in-place encode paths through it, giving
  future SIMD encode admission one audited integration point.
- Added the matching scalar-forced decode backend boundary for future decode
  admission symmetry, changed the scalar in-place encode invariant to return an
  error instead of panicking in release builds, tightened panic-policy scanning
  for production equality assertions, and clarified that in-place clear-tail
  errors intentionally clear original caller input.

## Commit Range

- Previous tag: `v1.1.6`
- Release tag: `v1.2.0`
- Release date: `2026-06-21`

## Commits

### Added

- `129e7ea` Admit AVX2 encode backend
- `ca97821` Admit AVX-512 encode backend
- `7deaeac` Admit AArch64 NEON encode backend
- `bceb678` Add AArch64 Linux verification helper
- `ff24eee` Clarify admitted SIMD cleanup labels
- `fc0cf38` Add locked sanitization decode helpers

### Security / Hardening

- `99f1d94` Harden SIMD performance evidence

### Documentation

- `8ad69f7` Stage workspace for 1.2.0 family release

### Verification

- `f7ed4d7` Record effective backend in perf evidence
- `d3d3dc1` Fix AArch64 NEON test helper visibility

### Other Changes

- `bdfd44a` Update sanitization companion to 1.1.0
- `9c7c49c` Gate SIMD encode by fixed block size
- `37051ac` Avoid temporary copies in SIMD slice encode
- `ed16364` Clarify staged SIMD encode scope
- `abe8a5c` Record AArch64 verification in changelog

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.2.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
