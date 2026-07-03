# base64-ng 1.0.3 Release Notes

Status: released

## Summary

- Continued the `1.0.x` source-layout series by splitting runtime backend
  reporting and backend-policy types into `src/runtime.rs` while preserving the
  public `base64_ng::runtime::*` API surface.
- Expanded GitHub Actions platform coverage with pinned macOS ARM runners
  (`macos-15`, `macos-26`) and a pinned Intel macOS runner
  (`macos-15-intel`) while keeping `macos-latest` as the moving-label signal.
- Added `scripts/check_macos.sh` for local macOS verification on Apple Silicon
  and Intel Macs, including host tests plus Apple Darwin target compile checks.
- Split alphabet definitions, custom alphabet validation, the alphabet macro,
  and `AlphabetError` into `src/alphabet.rs` while preserving all public root
  exports.
- Split `Profile` and the named MIME/PEM/bcrypt/crypt profile constants into
  `src/profiles.rs` while preserving all public root exports.
- Split best-effort cleanup and wipe helpers into `src/cleanup.rs`, preserving
  internal call paths and updating the unsafe-boundary release gate.

## Commit Range

- Previous tag: `v1.0.2`
- Release tag: `v1.0.3`
- Release date: `2026-05-29`

## Commits

### Added

- `1a56467` Add macOS verification script
- `38f14b4` Add macOS target std diagnostics

### Security / Hardening

- `0a521ae` Harden macOS target checks
- `44f0f0c` Harden macOS host target checks
- `477120a` Address 1.0.3 pentest cleanup findings

### Other Changes

- `965d809` Start 1.0.3 runtime module split
- `34b949f` Split alphabet module
- `4c0f12a` Split profiles module
- `fbaf978` Split cleanup module
- `1f8036f` Pin macOS script toolchain commands
- `527c781` Use rustup run in macOS checks
- `6492699` Pin rustc for macOS verification
- `5f3f498` Relax CT assembly symbol path checks
- `fee4afd` Keep secret string conversion panic-free

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.3.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
