# base64-ng 1.0.10 Release Notes

Status: released

## Summary

- Split oversized production modules into focused internal source files for the
  engine, buffers, constant-time helpers, streaming adapters, runtime reporting,
  SIMD scaffolding, Kani proofs, and unit tests. This is a source-layout
  maintenance release with no intended public API or behavior changes.
- Added `scripts/validate-file-line-budget.sh` and wired it into the normal
  checks so production Rust source under `src/` and companion crate sources stay
  within the 500-line maintainability budget.
- Added post-pentest hardening before tagging: `wipe_tail` now clamps after a
  debug-only invariant check instead of panicking in cleanup code, `DecodeError`
  `Debug` output is redacted to the error kind, and custom alphabet timing
  documentation now distinguishes the strict path from the `ct` scanner.
- Refreshed `base64-ng-serde` to `1.0.10` with drop-time cleanup for wrapper
  bytes and explicit comparison behavior; the other optional companion crates
  remain at `1.0.9`.

## Commit Range

- Previous tag: `v1.0.9`
- Release tag: `v1.0.10`
- Release date: `2026-06-20`

## Commits

### Added

- `6016c81` Add high-assurance deployment checklist
- `fe9a789` Add crate version matrix

### Security / Hardening

- `3349c3a` Address 1.0.10 pentest hardening

### Documentation

- `e192cae` Adjust publish flow after pre-tag release gate
- `f7cd5c5` Document companion crate cargo links

### Other Changes

- `5fe5ebd` Prepare 1.0.10 source layout maintenance
- `a3a576c` Ignore Python helper caches

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.10.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
