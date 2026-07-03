# base64-ng 1.2.1 Release Notes

Status: released

## Summary

- Bumped the workspace family to `1.2.1` so crates.io and docs.rs show the
  corrected `1.2.x` README, companion-crate examples, and release matrix.
- Updated README guidance for the completed `1.2.0` encode-acceleration
  release, including an explicit `simd` feature install snippet and an example
  showing that public encode APIs remain unchanged while runtime dispatch
  selects an admitted backend only when the platform and input shape qualify.
- Refreshed SIMD admission, roadmap, dependency, migration, and companion-crate
  documentation so `1.2.0` is no longer described as staged.

## Commit Range

- Previous tag: `v1.2.0`
- Release tag: `v1.2.1`
- Release date: `2026-06-21`

## Commits

### Documentation

- `d8a419b` Prepare 1.2.1 documentation patch

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.2.1.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
