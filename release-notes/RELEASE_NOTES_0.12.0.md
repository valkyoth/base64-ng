# base64-ng 0.12.0 Release Notes

Status: released

## Summary

- Started the stabilization rehearsal cycle after the `0.11.0` release.
- Added a migration-guide smoke crate and release-gate check covering strict
  standard, URL-safe no-pad, MIME/PEM, legacy whitespace, custom alphabet,
  stack-buffer, secret-buffer, and stream migration examples.
- Hardened release metadata validation and the stable release gate so the
  migration smoke source and check script stay packaged and release-gated.
- Added an MSRV/toolchain policy validator covering Cargo metadata,
  `rust-toolchain.toml`, docs.rs metadata, CI install paths, target matrices,
  Kani policy, and release evidence tooling.
- Added the `v0.12` final dependency admission review, keeping optional
  ecosystem integrations deferred unless they earn separate admission evidence.
- Changed custom alphabet byte decoding to scan all 64 alphabet entries before
  returning, avoiding match-position early returns for bcrypt-style,
  `crypt(3)`-style, and caller-defined alphabets.
- Clarified that default strict decoders are not constant-time decoders and
  that secret-bearing payloads should use the `ct` module when timing posture
  matters more than localized error diagnostics.
- Changed internal stream output-queue saturation errors away from
  `InvalidInput` so bounded queue exhaustion is not reported as malformed
  caller input.
- Expanded software-only wipe documentation with the known limits of volatile
  best-effort cleanup and the recommended application-owned `zeroize` pattern
  for deployments that already admit that dependency.

## Commit Range

- Previous tag: `v0.11.0`
- Release tag: `v0.12.0`
- Release date: `2026-05-17`

## Commits

### Added

- `3cd7bfa` Add migration guide smoke checks
- `efb1ea8` Add MSRV policy validation

### Security / Hardening

- `235d644` Address pentest timing and cleanup findings

### Documentation

- `c49fafa` Package migration smoke release checks
- `06e40da` Release 0.12.0

### Other Changes

- `febd862` Start 0.12 stabilization cycle
- `966b272` Record final dependency admission review

## Verification

This file is generated from repository history. See the matching
`security/pentest/v0.12.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
