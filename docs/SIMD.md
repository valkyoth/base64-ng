# SIMD Admission Policy

`base64-ng` is intentionally scalar-only in the `0.7.0` release and through
current `0.8.0-alpha.0` development unless a complete SIMD admission evidence
package lands in the same release series. The crate uses
`#![deny(unsafe_code)]` and permits reviewed `allow(unsafe_code)` exceptions
only for volatile wipe helpers in `src/lib.rs` and the private `src/simd.rs`
boundary. The `simd` feature remains reserved until architecture-specific code
has enough evidence to justify enabling it.

This is a security decision, not a rejection of hardware acceleration. SIMD
must be added only when it can be isolated, tested, and reviewed without
weakening the scalar trust base.

## Current Status

- Default builds compile audited unsafe volatile wipe helpers; scalar
  encode/decode remains safe Rust.
- `scripts/validate-unsafe-boundary.sh` verifies that `allow(unsafe_code)` is
  confined to the volatile wipe helpers and `src/simd.rs`.
- `docs/UNSAFE.md` inventories every current unsafe site and its invariants.
- The scalar implementation is the reference behavior.
- Encode and decode entry points already pass through an internal backend
  boundary, currently backed only by the scalar implementation.
- With the `simd` feature enabled, the private dispatch scaffold detects
  AVX-512 VBMI, AVX2, SSSE3/SSE4.1, NEON, and wasm `simd128` candidates but
  still activates only the scalar backend.
- AVX-512 VBMI detection is reporting-only until an implementation has scalar
  differential tests, fuzz coverage, and benchmark evidence. Detection requires
  the full planned feature bundle: `avx512f`, `avx512bw`, `avx512vl`, and
  `avx512vbmi`.
- An inactive AVX-512 fixed-block encode prototype exists behind the SIMD
  boundary and is tested against scalar output only when the full AVX-512
  Base64 feature bundle is available.
- Runtime backend identifiers expose their required CPU feature bundles through
  `runtime::Backend::required_cpu_features()`.
- Runtime backend reports include `candidate_required_cpu_features=[...]` in
  their stable key/value display output for audit logs.
- Runtime backend reports expose `snapshot()` for structured audit logging
  without parsing formatted strings.
- SSSE3/SSE4.1 detection is reporting-only until an implementation has scalar
  differential tests, fuzz coverage, and benchmark evidence.
- An inactive SSSE3/SSE4.1 fixed-block encode prototype exists behind the SIMD
  boundary and is tested against scalar output only when SSSE3/SSE4.1 is
  available.
- An inactive AVX2 fixed-block encode prototype exists behind the SIMD boundary
  and is tested against scalar output only when AVX2 is available.
- An inactive NEON fixed-block encode prototype exists behind the same boundary
  and is tested against scalar output only on NEON-capable ARM targets.
- wasm `simd128` detection is reporting-only when `wasm32` is compiled with
  `target-feature=+simd128`; no wasm accelerated backend is active.
- `runtime::backend_report()` reports the active backend, detected candidate,
  SIMD feature status, and scalar-only security posture.
- `runtime::require_backend_policy()` allows deployments to enforce scalar
  execution, disabled SIMD features, or no detected SIMD candidate.
- `BackendPolicy::HighAssuranceScalarOnly` combines scalar execution, disabled
  SIMD features, no detected SIMD candidate, and unsafe-boundary enforcement.
- Runtime backend, posture, and policy enums provide stable string identifiers
  for logs and release evidence.
- Runtime backend reports and policy failures format as stable key/value
  strings suitable for CI and audit logs.
- Unit tests compare dispatch behavior against the scalar reference for
  canonical inputs, malformed inputs, and undersized output buffers.
- The `simd` feature does not enable accelerated code yet.
- Current `0.8` development remains scalar-only unless the SIMD admission
  manifest, scalar differential tests, fuzz evidence, unsafe inventory,
  architecture evidence, benchmark evidence, and release wording are updated
  together.
- CI checks the reserved `simd` feature in `no_std` mode for x86_64, aarch64,
  FreeBSD, wasm32, and Cortex-M targets.
- Performance claims must be backed by local benchmark evidence, not roadmap
  language.

Run the same target check locally for every installed target:

```sh
scripts/check_targets.sh
```

Run a specific target:

```sh
scripts/check_targets.sh aarch64-unknown-linux-gnu
```

Compile-check the reserved SIMD feature bundles:

```sh
scripts/check_simd_feature_bundles.sh
```

This does not execute accelerated code. It proves the reserved AVX2,
AVX-512, SSSE3/SSE4.1, NEON, and wasm `simd128` feature-gated code still
compiles under `no_std` when the corresponding Rust targets are installed.

Capture local backend and prototype evidence:

```sh
scripts/check_backend_evidence.sh
```

This prints the runtime backend-report test and runs the gated SIMD prototype
scalar-equivalence tests with `--nocapture`, so local CPU evidence is easy to
copy into release notes or issue discussion. It also writes
`target/release-evidence/backend/MANIFEST.txt` with toolchain metadata,
commands, status values, and artifact checksums.

## Required Before SIMD Code Lands

Any AVX2, NEON, AVX-512, wasm `simd128`, or runtime-dispatch implementation
must include:

- The dedicated `src/simd.rs` boundary for all architecture-specific code.
- Crate-level `deny(unsafe_code)` must continue to reject unsafe outside the
  volatile wipe helpers and SIMD module.
- A local safety comment for every unsafe block.
- Deterministic differential tests against scalar encode/decode behavior.
- Fuzz differential coverage for strict and legacy-compatible inputs where
  applicable.
- Runtime dispatch tests that prove unsupported CPUs fall back to scalar.
- Miri coverage for scalar and dispatch-level code that Miri can execute.
- Architecture-specific CI evidence or documented local evidence for each
  enabled target.
- Benchmark evidence that reports hardware, OS, Rust version, command, and raw
  output.

## Admission Gate

`scripts/validate-simd-admission.sh` keeps SIMD dispatch scalar-only until the
admission evidence is deliberately updated. The gate currently requires:

- `ActiveBackend` to expose only the `Scalar` variant.
- `active_backend()` to return `ActiveBackend::Scalar`.
- No accelerated `ActiveBackend::Avx*`, `ActiveBackend::Neon`,
  `ActiveBackend::Sse*`, `ActiveBackend::Wasm*`, or generic SIMD dispatch
  variants in source.
- `docs/SIMD_ADMISSION.md` to record that no accelerated backend is admitted.
- Documentation for benchmark evidence, release-note restrictions, and
  vector-register retention cleanup strategy to remain packaged.

When an accelerated backend is ready for admission, update this gate in the
same commit as the scalar differential tests, fuzz evidence, unsafe inventory,
benchmark evidence, and release notes.

## Dispatch Rules

- Scalar remains the fallback for every build.
- Candidate detection must not imply activation; a detected candidate may still
  execute scalar until the accelerated backend is admitted.
- Prototype functions may exercise target-feature and unsafe plumbing without
  being eligible for dispatch.
- Runtime CPU detection may be used only behind `std`.
- Compile-time target-feature paths must be explicit and documented.
- Unsupported CPU features must never panic at runtime.
- SIMD paths must preserve strict error indexes, canonical padding rejection,
  and output sizing behavior.

## Release Rule

Do not advertise SIMD acceleration in release notes until accelerated code is
actually enabled, tested, and measured for that release.
