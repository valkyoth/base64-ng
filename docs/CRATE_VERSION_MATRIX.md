# Crate Version Matrix

This matrix tracks the `base64-ng` crate family for release review. The core
crate remains the stable user entry point; companion crates are optional
integration packages for applications that explicitly admit their dependency
sets.

## 1.3.3 Release Plan

The `1.3.3` release keeps the workspace crate family synchronized. The main
crate carries the narrow wasm `simd128` runtime-dispatch admission, Node/V8,
Wasmtime, Chromium-family browser, and Firefox/SpiderMonkey smoke evidence,
release-gated wasm codegen evidence, and wrapped-profile helper ergonomics. Companion crates
receive synchronized package metadata so downstream users see one coherent
crate-family version.

| Crate | Version | Publish In 1.3.3 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.3` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.3` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.3` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.3` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.3` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.3` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.3` | yes | <https://crates.io/crates/base64-ng-tokio> |

## 1.3.2 Release Plan

The `1.3.2` release keeps the workspace crate family synchronized. The main
crate carries the non-standard SIMD surface review, test-only evidence, and
release-governance updates. Companion crates receive synchronized package
metadata so downstream users see one coherent crate-family version.

| Crate | Version | Publish In 1.3.2 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.2` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.2` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.2` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.2` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.2` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.2` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.2` | yes | <https://crates.io/crates/base64-ng-tokio> |

## 1.3.1 Release Plan

The `1.3.1` release kept the workspace crate family synchronized. The main
crate and `base64-ng-tokio` carried the async writer patch; the remaining
companion crates received synchronized package metadata so downstream users saw
one coherent crate-family version.

| Crate | Version | Publish In 1.3.1 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.1` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.1` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.1` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.1` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.1` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.1` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.1` | yes | <https://crates.io/crates/base64-ng-tokio> |

## 1.3.0 Release Plan

The `1.3.0` family is published as a synchronized implementation-completion
release after the final release gate, Kani, hardware checks, GitHub CI, and
external review are clean.

| Crate | Version | Publish In 1.3.0 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.0` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.0` | yes | <https://crates.io/crates/base64-ng-tokio> |

## Release Policy

- Publish `base64-ng` first when the core crate changes.
- Publish companion crates only when their own code, documentation, or public
  dependency requirement changes.
- Keep unchanged companion crates on their last published patch version; Cargo's
  normal compatible version ranges allow them to resolve with newer compatible
  `base64-ng` releases.
- Review companion crate dependencies at the integration layer. The core
  `base64-ng` package keeps its zero-runtime-dependency policy.
