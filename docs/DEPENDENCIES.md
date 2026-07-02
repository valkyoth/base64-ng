# Dependency Admission Policy

`base64-ng` defaults to zero external crates in the published package. That is
a security and maintenance choice: Base64 is infrastructure code, and every
new dependency expands the audit, license, advisory, and supply-chain surface.

## Current Status

- `Cargo.toml` has no normal, build, or dev dependencies.
- `scripts/validate-dependencies.sh` fails if the root crate dependency graph
  contains anything beyond `base64-ng` itself.
- `scripts/check_reserved_features.sh` verifies that `tokio`, `kani`, and
  `fuzzing` remain inert and dependency-free until admitted, and that deferred
  integration features such as `serde`, `bytes`, `zeroize`, `subtle`, and
  `criterion` are not exposed before dependency admission.
- `allow-wasm32-best-effort-wipe` is a dependency-free policy feature, not a
  dependency admission. It is required to build for `wasm32`, where cleanup is
  limited to a compiler-fence-only wipe barrier.
- `allow-compiler-fence-only-wipe` is a dependency-free policy feature, not a
  dependency admission. It is required to build unsupported native architectures
  that do not have a `base64-ng` hardware wipe barrier and therefore fall back
  to compiler-fence-only cleanup.
- `base64_ng_aarch64_csdb_attested` is a dependency-free custom cfg operator
  attestation, not a dependency admission and not a Cargo feature. It should
  only be enabled after the deployment has evidence that the target AArch64
  core treats CSDB as an effective speculation barrier for the CT result gate.
  Builds that set it report `hardware-speculation-barrier-build-asserted` so
  audit logs preserve the operator-attestation boundary.
- `base64-ng-sanitization` is an optional companion package for applications
  that already admit `sanitization`; it is not a dependency of the core
  `base64-ng` package. Its `1.2.x` line requires an exact
  `sanitization` `=1.2.2` dependency
  so callers can use `sanitization::ct::Choice`, native
  constant-time-oriented equality helpers, and opt-in locked-secret fill APIs
  without adding dependencies to the core crate. Release review must verify
  the crates.io owner set for `sanitization` before publishing companion
  updates because that crate sits directly in the optional secret-cleanup
  dependency chain.
- `base64-ng-derive` is an optional companion package for fixed-size byte
  newtypes. It is dependency-free and does not add proc-macro machinery to the
  core `base64-ng` package.
- `base64-ng-serde`, `base64-ng-bytes`, `base64-ng-subtle`, and
  `base64-ng-tokio` are optional companion packages for applications that
  already admit `serde`, `bytes`, `subtle`, or `tokio`; they are not
  dependencies of the core `base64-ng` package.
- Fuzz, performance, and dudect-style timing harness dependencies are isolated
  under `fuzz/`, `perf/`, and `dudect/`; the standard local gate checks them
  separately from the published crate dependency graph.
- CI toolchain setup requires `rustup` and `cargo` from the runner image. The
  repository script intentionally refuses unauthenticated `curl | sh` rustup
  bootstrap during CI; missing toolchain managers are treated as infrastructure
  failures, not as a reason to execute freshly downloaded shell installers.

## v1.0 Final Admission Review

The `v1.0` release keeps the core `base64-ng` package dependency-free.
Optional ecosystem integrations may be admitted only as separate companion
crates with their own dependency review and release checks.

Current decisions:

- `base64-ng-sanitization` is admitted as a companion crate because it keeps the
  core package dependency-free while giving applications that already use
  `sanitization` a direct CT decode path into clear-on-drop secret containers.
  Its optional `high-assurance` feature admits `sanitization` memory-locking,
  canary-check, and random-canary features so supported native deployments can
  decode directly into locked mappings through `LockedSecretBytes` or
  `LockedSecretVec`.
- `base64-ng-derive` is admitted as a companion crate because it keeps
  proc-macro code and generated newtype ergonomics outside the core package.
  The derive surface is intentionally limited to tuple structs with one
  `[u8; N]` field.
- `base64-ng-serde` is admitted as a companion crate because serialization
  remains explicit at the field boundary and does not hide alphabet or padding
  choices inside the core package.
- `base64-ng-bytes` is admitted as a companion crate because services that
  already use `bytes` can opt into `Bytes`, `Buf`, and `BufMut` helpers without
  adding `bytes` to the core package.
- `base64-ng-subtle` is admitted as a companion crate because authentication,
  MAC, password-hash, and token verification boundaries can opt into a reviewed
  `subtle::ConstantTimeEq` primitive without adding `subtle` to the core
  package.
- `base64-ng-tokio` is admitted as a companion crate for async read-all/write-all
  helpers, including caller-limited variants for peer-controlled request or
  frame boundaries, and manual `AsyncRead` streaming adapters with fixed
  buffers and drop cleanup. Async writer state machines remain deferred until
  cancellation-safety, accepted-byte, backpressure, and drop-cleanup evidence is
  complete.
- The core `tokio` feature remains reserved and inert until async
  cancellation, drop cleanup, chunk-boundary, dependency, and release-evidence
  requirements are satisfied.
- `zeroize` remains deferred for the core crate; applications can combine their
  own approved dependencies with caller-owned buffers while `base64-ng` keeps
  its audited local best-effort helpers dependency-free.
- `subtle` is admitted only through `base64-ng-subtle`, not through the core
  crate.
- Property-testing and benchmark frameworks remain isolated or deferred; fuzz,
  dudect-style timing, and performance harnesses stay outside the published
  crate package.

## Admission Requirements

Before adding any dependency to the published crate, the change must document:

- Why `core`, `alloc`, or `std` is not sufficient.
- Whether the dependency is runtime, build-time, dev-only, feature-gated, or
  tool-only.
- The full transitive dependency graph.
- License compatibility with `MIT OR Apache-2.0`.
- RustSec advisory status and yanked-release status.
- Whether the dependency works under the crate's supported `no_std` feature
  combinations.
- Whether the dependency changes MSRV, build reproducibility, or target support.
- How the dependency is disabled for users who do not need the feature.

The release gate must remain clean after the change:

```sh
scripts/checks.sh
scripts/stable_release_gate.sh release
```

At minimum, evidence must include:

- `cargo tree` for the affected feature set.
- `cargo deny check`.
- `cargo audit`.
- `cargo license --json`.
- Updated release notes and migration/security documentation when the public
  API or threat model changes.

## Default Rejections

The following are rejected unless a specific review proves they are necessary:

- Helper crates for small bit manipulation, table generation, feature
  selection, error formatting, or simple CLI behavior.
- Git dependencies.
- Default-feature runtime dependencies.
- Dependencies with unclear licensing, unmaintained status, active security
  advisories, yanked releases, or unnecessary transitive graphs.

## Deferred Core Integrations

The following integrations are intentionally not admitted in the published
core crate today:

- `tokio`: the core feature remains reserved and inert. Use `base64-ng-tokio`
  for the admitted read-all/write-all helper surface and read-side streaming
  adapters. Prefer caller-limited helpers for peer-controlled input.
- `serde`: use `base64-ng-serde` when explicit serialization wrappers or
  field-level modules for Standard, URL-safe, MIME, or PEM profiles are
  needed. The core crate does not admit `serde`.
- `bytes`: use `base64-ng-bytes` when `Bytes`, `Buf`, or `BufMut` integration
  is needed. The core crate does not admit `bytes`.
- `zeroize`: deferred unless a review proves that the dependency materially
  improves the documented best-effort cleanup posture beyond the current
  audited local helpers.
- `subtle`: use `base64-ng-subtle` when protocol code needs a reviewed
  constant-time equality primitive for decoded or encoded buffers.
- Criterion or other benchmark frameworks: keep benchmark evidence isolated
  unless the added dependency graph clearly improves release evidence quality.

These are product decisions as much as technical ones. The crate is allowed to
remain smaller than the broader ecosystem when dependency-free APIs preserve
explicit security semantics.

Downstream applications may still combine `base64-ng` with their own approved
dependencies. For example, a service with an existing `zeroize` policy can
decode into a caller-owned buffer with `decode_slice_clear_tail` and then call
`Zeroize::zeroize()` on that buffer after the protocol step is complete. That
keeps the published `base64-ng` crate dependency-free while allowing the
application to apply its local memory-cleanup policy at the ownership boundary.
This is the recommended pattern for deployments that require a dependency-backed
zeroization policy while still wanting `base64-ng` itself to remain a small,
auditable zero-runtime-dependency crate.

## Isolated Tooling

Fuzzing, benchmark, and timing-evidence dependencies may live in isolated
workspaces only when they are not packaged with the published crate:

- `fuzz/` dependencies are reviewed by `scripts/check_fuzz.sh`.
- `perf/` dependencies are reviewed by `scripts/check_perf.sh`.
- `dudect/` dependencies are reviewed by `scripts/check_dudect.sh`.
- `crates/base64-ng-sanitization/`, `crates/base64-ng-derive/`,
  `crates/base64-ng-serde/`, `crates/base64-ng-bytes/`,
  `crates/base64-ng-subtle/`, and `crates/base64-ng-tokio/` are optional
  companion crates, not dependencies of the core `base64-ng` package. They are
  reviewed separately by `scripts/check_companion_crates.sh` so the root package keeps its
  zero-runtime-dependency guarantee.

`scripts/checks.sh` runs those isolated harness checks so ordinary local
verification catches harness dependency drift before release-only evidence
steps.

Those isolated dependencies do not weaken the zero-dependency guarantee for the
published core crate.
