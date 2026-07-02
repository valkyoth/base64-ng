# Crate Version Matrix

This matrix tracks the `base64-ng` crate family for release review. The core
crate remains the stable user entry point; companion crates are optional
integration packages for applications that explicitly admit their dependency
sets.

## Upcoming 1.3.0 Release Candidate Plan

The working branch is preparing a synchronized `1.3.0` family after the final
release gate, Kani, hardware checks, GitHub CI, and external review are clean.
Do not publish this matrix until package versions are actually bumped to
`1.3.0` in the final release-candidate commit.

| Crate | Planned Version | Publish In 1.3.0 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.0` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-tokio> |

## 1.2.3 Sanitization Dependency Sync Release Plan

| Crate | Version | Publish In 1.2.3 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.2.3` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.2.3` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.2.3` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.2.3` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.2.3` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.2.3` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.2.3` | yes | <https://crates.io/crates/base64-ng-tokio> |

## Release Policy

- Publish `base64-ng` first when the core crate changes.
- Publish companion crates only when their own code, documentation, or public
  dependency requirement changes.
- Keep unchanged companion crates on their last published patch version; Cargo's
  normal compatible version ranges allow them to resolve with newer compatible
  `base64-ng` releases.
- Review companion crate dependencies at the integration layer. The core
  `base64-ng` package keeps its zero-runtime-dependency policy.
