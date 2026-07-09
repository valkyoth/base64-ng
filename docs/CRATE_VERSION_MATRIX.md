# Crate Version Matrix

This matrix tracks the `base64-ng` crate family for release review. The core
crate remains the stable user entry point; companion crates are optional
integration packages for applications that explicitly admit their dependency
sets.

## 1.3.6 Release Plan

The `1.3.6` release keeps the workspace crate family synchronized. The main
crate and all companion crates carry the companion README header refresh so
crate pages share the project image, core documentation links, and a
crate-specific one-line summary. No encode/decode logic, SIMD admission scope,
or runtime dependency posture changes in this release.

| Crate | Version | Publish In 1.3.6 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.6` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.6` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.6` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.6` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.6` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.6` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.6` | yes | <https://crates.io/crates/base64-ng-tokio> |

## 1.3.5 Release Plan

The `1.3.5` release keeps the workspace crate family synchronized. The main
crate carries the RISC-V QEMU scalar/fallback evidence line, required
`riscv64gc-unknown-linux-gnu` QEMU coverage, stable Rust
`riscv_ext_intrinsics` blocker checks, and explicit documentation that RISC-V
RVV acceleration is not admitted until real hardware evidence and a reviewed
stable-intrinsic or assembly-backed backend exist. Companion crates receive
synchronized package metadata so downstream users see one coherent crate-family
version.

| Crate | Version | Publish In 1.3.5 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.5` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.5` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.5` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.5` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.5` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.5` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.5` | yes | <https://crates.io/crates/base64-ng-tokio> |

## 1.3.4 Release Plan

The `1.3.4` release keeps the workspace crate family synchronized. The main
crate carries the big-endian QEMU scalar/fallback evidence line, required
`s390x-unknown-linux-gnu` QEMU coverage, stable Rust `s390x`/PowerPC64
intrinsic blocker checks, and explicit documentation that big-endian
acceleration is not admitted until stable intrinsics or a separate
assembly-backed review exists. Companion crates receive synchronized package
metadata so downstream users see one coherent crate-family version.

| Crate | Version | Publish In 1.3.4 | Cargo |
| --- | --- | --- | --- |
| `base64-ng` | `1.3.4` | yes | <https://crates.io/crates/base64-ng> |
| `base64-ng-sanitization` | `1.3.4` | yes | <https://crates.io/crates/base64-ng-sanitization> |
| `base64-ng-derive` | `1.3.4` | yes | <https://crates.io/crates/base64-ng-derive> |
| `base64-ng-serde` | `1.3.4` | yes | <https://crates.io/crates/base64-ng-serde> |
| `base64-ng-bytes` | `1.3.4` | yes | <https://crates.io/crates/base64-ng-bytes> |
| `base64-ng-subtle` | `1.3.4` | yes | <https://crates.io/crates/base64-ng-subtle> |
| `base64-ng-tokio` | `1.3.4` | yes | <https://crates.io/crates/base64-ng-tokio> |

## 1.3.3 Release Plan

The `1.3.3` release keeps the workspace crate family synchronized. The main
crate carries the narrow wasm `simd128` runtime-dispatch admission, Node/V8,
Wasmtime, Chromium-family browser, Firefox/SpiderMonkey, and Safari/WebKit
smoke evidence, release-gated wasm codegen evidence, wrapped-profile helper
ergonomics, wrapped/legacy decode staging, and stack-staged in-place
encode/decode admission. Companion crates receive synchronized package
metadata so downstream users see one coherent crate-family version.

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
