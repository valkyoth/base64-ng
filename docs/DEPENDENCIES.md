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
- Fuzz, performance, and dudect-style timing harness dependencies are isolated
  under `fuzz/`, `perf/`, and `dudect/`; the standard local gate checks them
  separately from the published crate dependency graph.

## v1.0 Final Admission Review

The `v1.0` release candidate keeps the published crate dependency-free.
No optional ecosystem integration has a strong enough security and maintenance
case to enter the stable `v1.0` contract yet.

Current decisions:

- `tokio` remains a reserved, inert feature until async cancellation, drop
  cleanup, chunk-boundary, dependency, and release-evidence requirements are
  satisfied.
- `serde` remains deferred because serialization should not hide alphabet,
  padding, profile, allocation, or secret-handling choices.
- `bytes` remains deferred because slice, stack-buffer, and `std::io` surfaces
  cover current networking and streaming use cases without a runtime
  dependency.
- `zeroize` and `subtle` remain deferred; applications can combine their own
  approved dependencies with caller-owned buffers while `base64-ng` keeps its
  audited local best-effort helpers dependency-free.
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

## Deferred Integrations

The following integrations are intentionally not admitted in the published
crate today:

- `tokio`: reserved for async streaming only after the policy in
  [`ASYNC.md`](ASYNC.md) is satisfied.
- `serde`: deferred until a concrete serialization use case proves that native
  `AsRef`, `TryFrom`, engine/profile APIs, and caller-owned buffers are
  insufficient.
- `bytes`: deferred until a concrete networking use case proves that slice and
  `std::io` APIs are insufficient.
- `zeroize` or `subtle`: deferred unless a review proves that the dependency
  materially improves the documented best-effort cleanup or
  constant-time-oriented posture beyond the current audited local helpers.
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

`scripts/checks.sh` runs those isolated harness checks so ordinary local
verification catches harness dependency drift before release-only evidence
steps.

Those isolated dependencies do not weaken the zero-dependency guarantee for the
published crate.
