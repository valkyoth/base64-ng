# base64-ng 1.3.2 Release Notes

`1.3.2` publishes the full `base64-ng` crate family as a synchronized
non-standard SIMD surface review release. It does not admit any new runtime
SIMD acceleration and does not change production encode, decode, SIMD,
constant-time, cleanup, or dependency code.

## Added

- Added `docs/SIMD_NON_STANDARD_SURFACE_REVIEW.md`, an admission ledger for
  custom alphabets, bcrypt-style and `crypt(3)` profiles, wrapped encode and
  decode, legacy-whitespace decode, in-place encode and decode, and
  constant-time-oriented secret decode.
- Added test-only evidence for non-standard SIMD candidate surfaces:
  successful behavior, malformed/error behavior, clear-tail behavior,
  in-place encode parity, and named profile forwarding.
- Added an independent naive wrapped-output oracle for wrapped encode tests.
- Added `scripts/validate-simd-non-standard-surfaces.sh` and wired it into the
  normal check gate.

## Security And Policy

- No new non-standard SIMD acceleration is admitted in this release.
- Named profiles (`MIME`, `PEM`, `PEM_CRLF`, `BCRYPT`, and `CRYPT`) are
  documented and tested as convenience forwarding surfaces, not separate SIMD
  admission scopes.
- Panic-policy validation now proves root `src/*_tests.rs` files are declared
  behind `#[cfg(test)]` before exempting them from test-only panic allowances.
- External review found no shipped-code findings for the `v1.3.1..HEAD`
  range because production runtime code and dependency metadata did not change
  before the final version-sync commit.

## Published Crates

- `base64-ng` `1.3.2`
- `base64-ng-sanitization` `1.3.2`
- `base64-ng-derive` `1.3.2`
- `base64-ng-serde` `1.3.2`
- `base64-ng-bytes` `1.3.2`
- `base64-ng-subtle` `1.3.2`
- `base64-ng-tokio` `1.3.2`

## Verification

- `cargo test --all-features non_standard -- --nocapture`
- `scripts/validate-simd-non-standard-surfaces.sh`
- `scripts/validate-panic-policy.sh`
- `cargo clippy --all-features --all-targets -- -D warnings`
- `cargo fmt --all --check`
- `scripts/checks.sh`

## Tag

- Release tag: `v1.3.2`
