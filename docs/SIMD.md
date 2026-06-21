# SIMD Admission Policy

`base64-ng` is intentionally scalar-only throughout the `1.1.x` line. Future SIMD
dispatch remains gated unless a complete SIMD admission evidence package lands
in a later release series. The crate uses `#![deny(unsafe_code)]` and permits
reviewed `allow(unsafe_code)` exceptions only for audited cleanup in
`src/cleanup.rs`, CT comparison, byte accumulation, CT scan, and CT result-gate
helpers in `src/ct/`, and the private `src/simd/` boundary. The `simd` feature
remains reserved until architecture-specific code has enough evidence to
justify enabling it.

This is a security decision, not a rejection of hardware acceleration. SIMD
must be added only when it can be isolated, tested, and reviewed without
weakening the scalar trust base.

## Version Roadmap

The SIMD roadmap separates implementation evidence from active acceleration:

- `1.1.x` is the SIMD encode foundation and admission-candidate series. Early
  checkpoints contain real fixed-block encode prototypes for SSSE3/SSE4.1,
  AVX2, AVX-512 VBMI, NEON, and wasm `simd128`, plus scalar-equivalence tests,
  generated assembly evidence, register-cleanup review, fuzz expansion, and
  admission-tooling updates. Later checkpoints wire admitted encode backends
  into public encode APIs while keeping each checkpoint gated by pentest, CI,
  and release evidence. GitHub checkpoint tags in this line may move evidence
  forward without a matching crates.io publish; the next planned crates.io
  family sync is `1.2.0`.
- `1.1.5` adds the public encode backend boundary while still forcing scalar
  execution. This gives future accelerated encode admission one reviewed
  integration point for `encode_slice`, clear-tail helpers, alloc helpers,
  wrapped helpers, and in-place encode.
- `1.2.0` is the release where encode acceleration must be fully working for
  the admitted encode scope. Public encode APIs must dispatch to admitted
  encode backends when runtime policy and CPU features allow it, and must fall
  back to scalar for unsupported CPUs, `no_std`, custom alphabets unless
  separately admitted, wrapping, legacy profiles, tails, and padding. Backends
  without complete evidence remain real non-dispatchable prototypes. The draft
  package for that decision lives in
  [SIMD_ENCODE_ADMISSION_DRAFT.md](SIMD_ENCODE_ADMISSION_DRAFT.md).
- After `1.2.0`, pause feature work for a short soak period so users can report
  platform-specific encode regressions before decode acceleration work starts.
- `1.2.x` is the SIMD decode foundation series. Decode prototypes remain
  non-dispatchable while invalid-input handling, canonicality, padding, output
  retention, error behavior, fuzz coverage, and timing-oriented evidence are
  proven against scalar behavior.
- `1.3.0` is the first release that may activate SIMD decode acceleration if
  the `1.2.x` decode evidence line is complete and the encode acceleration line
  has remained stable.

Patch releases in the `1.1.x` and `1.2.x` series may be small by design. Each
patch should move one evidence boundary forward without changing the active
runtime behavior for that line.

## Current Status

- Default builds compile audited unsafe cleanup, CT barrier, and comparison
  helpers; scalar encode/decode remains safe Rust.
- `scripts/validate-unsafe-boundary.sh` verifies that `allow(unsafe_code)` is
  confined to the reviewed cleanup, CT, and SIMD helper files.
- `docs/UNSAFE.md` inventories every current unsafe site and its invariants.
- The scalar implementation is the reference behavior.
- Encode and decode entry points already pass through an internal backend
  boundary, currently backed only by the scalar implementation.
- With the `simd` feature enabled, the private dispatch scaffold detects
  AVX-512 VBMI, AVX2, SSSE3/SSE4.1, NEON, and wasm `simd128` candidates but
  still activates only the scalar backend.
- AVX-512 VBMI detection is reporting-only until the implementation has full
  admission evidence. Detection requires the planned feature bundle:
  `avx512f`, `avx512bw`, `avx512vl`, and `avx512vbmi`.
- An inactive AVX-512 VBMI fixed-block encode prototype exists behind the SIMD
  boundary as real non-dispatchable vector encode evidence for all alphabets.
  It uses AVX-512 lane-local byte shuffling, vector shifts/masks, and VBMI
  byte permutes over the alphabet table for fixed 48-byte input blocks, then
  clears ZMM/YMM state before returning. It is tested against scalar output
  only when the full AVX-512 Base64 feature bundle is available and is not
  reachable from runtime backend selection.
- Runtime backend identifiers expose their required CPU feature bundles through
  `runtime::Backend::required_cpu_features()`.
- Runtime backend reports include `candidate_required_cpu_features=[...]` in
  their stable key/value display output for audit logs.
- Runtime backend reports include `candidate_detection_mode=...` so logs show
  whether a SIMD candidate came from runtime CPU feature probing or from
  compile-time target features.
- Runtime backend reports expose `snapshot()` for structured audit logging
  without parsing formatted strings.
- SSSE3/SSE4.1 detection is reporting-only until an implementation has scalar
  differential tests, fuzz coverage, and benchmark evidence.
- An inactive SSSE3/SSE4.1 fixed-block encode prototype exists behind the SIMD
  boundary as real non-dispatchable vector encode evidence for Standard and
  URL-safe alphabets. It uses SSSE3 byte shuffling, SSE lane shifts/masks, and
  SSE4.1 byte blending for fixed 12-byte input blocks, then clears XMM
  registers before returning. It is tested against scalar output only when
  SSSE3/SSE4.1 is available and is not reachable from runtime backend
  selection.
- An inactive AVX2 fixed-block encode prototype exists behind the SIMD boundary
  as real non-dispatchable vector encode evidence for Standard and URL-safe
  alphabets. It uses AVX2 lane-local byte shuffling, vector shifts/masks, and
  byte blending for fixed 24-byte input blocks, then clears XMM/YMM state
  before returning. It is tested against scalar output only when AVX2 is
  available and is not reachable from runtime backend selection.
- An inactive AArch64 NEON fixed-block encode prototype exists behind the same
  boundary as real non-dispatchable vector encode evidence for Standard and
  URL-safe alphabets. It uses NEON table lookup, vector shifts/masks, and
  byte-select alphabet mapping for fixed 12-byte input blocks, then clears used
  NEON registers before returning. Custom alphabets and 32-bit `arm+neon`
  remain scalar scaffold paths. Runtime backend selection remains scalar-only.
- An inactive wasm `simd128` fixed-block encode prototype exists behind the
  same boundary as real non-dispatchable vector encode evidence for Standard
  and URL-safe alphabets. It uses wasm byte shuffling, vector shifts/masks, and
  branchless Standard-family alphabet mapping for fixed 12-byte input blocks.
  Custom alphabets remain scalar scaffold paths because portable wasm SIMD does
  not provide a direct 64-byte alphabet lookup. The wasm feature-bundle check
  builds wasm test binaries with `target-feature=+simd128`; this is compile and
  codegen evidence only, not a runtime/JIT timing or register-retention claim.
  Runtime backend selection remains scalar-only.
- `runtime::backend_report()` reports the active backend, detected candidate,
  detection mode, SIMD feature status, scalar-only security posture, and a
  conservative unsafe-boundary posture flag. The flag is true only when the
  reserved `simd` feature is disabled; SIMD-enabled builds include additional
  private prototype boundaries and must use the release evidence scripts for
  boundary validation.
- On `x86`/`x86_64` with `std`, candidate detection uses
  `std::is_x86_feature_detected!` runtime CPU probing. On `no_std`, wasm, and
  current ARM builds, candidate detection is compile-time target-feature
  reporting. A binary compiled with `-C target-feature=+avx2` can therefore
  report an AVX2 candidate even if it is deployed on a CPU that cannot execute
  AVX2 instructions. No SIMD dispatch is active today; any future `no_std`
  SIMD activation must require an explicit caller-side CPU contract or remain
  disabled where runtime probing is unavailable.
- `runtime::require_backend_policy()` allows deployments to enforce scalar
  execution, disabled SIMD features, or no detected SIMD candidate.
- `BackendPolicy::HighAssuranceScalarOnly` combines scalar execution, disabled
  SIMD features, no detected SIMD candidate, unsafe-boundary enforcement, and a
  CT result gate classified as an attested hardware speculation barrier. It
  rejects targets that report an unattested hardware barrier, ordering fence,
  or compiler fence. On AArch64, the crate emits `isb sy` plus CSDB hint code
  but reports `hardware-speculation-barrier-unattested` because deployments
  must attest whether that hint is effective on their specific core. Builds
  using the explicit `base64_ng_aarch64_csdb_attested` cfg report
  `hardware-speculation-barrier-build-asserted` so audit logs show the posture
  came from deployment evidence rather than a native target guarantee. On RISC-V,
  the reported CT gate is intentionally only `ordering-fence`; the base ISA
  does not provide a canonical Spectre-v1 speculation barrier, so
  platform-level mitigations are required for that threat model.
- Runtime backend, posture, and policy enums provide stable string identifiers
  for logs and release evidence.
- Runtime backend reports and policy failures format as stable key/value
  strings suitable for CI and audit logs.
- Unit tests compare dispatch behavior against the scalar reference for
  canonical inputs, malformed inputs, and undersized output buffers.
- The `simd` feature does not enable accelerated code yet.
- Current `1.1` development remains scalar-only unless the SIMD admission
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
compiles under `no_std` when the corresponding Rust targets are installed. For
wasm `simd128`, it also builds the wasm test binaries with `simd128` enabled so
the inactive prototype body is checked without requiring a wasm runtime.

Capture local backend and prototype evidence:

```sh
scripts/check_backend_evidence.sh
```

This prints the runtime backend-report test and runs the gated SIMD prototype
scalar-equivalence tests with `--nocapture`, so local CPU evidence is easy to
copy into release notes or issue discussion. On x86/x86_64 hosts with the
required feature bundles, the x86 tests exercise the inactive fixed-block
vector encode prototypes against scalar output. On AArch64 NEON-capable hosts,
the NEON test exercises the inactive fixed-block vector prototype for Standard
and URL-safe alphabets; 32-bit ARM remains scaffold evidence. The script also writes
`target/release-evidence/backend/MANIFEST.txt` with toolchain metadata,
commands, status values, artifact checksums, and explicit
`prototype_state=real-non-dispatchable` /
`active_backend_admitted=false` labels.

Capture generated assembly evidence for the inactive x86 encode prototypes:

```sh
scripts/generate_simd_asm_evidence.sh
```

The script emits release test-harness assembly for SSSE3/SSE4.1, AVX2, and
AVX-512 VBMI feature bundles and checks for expected vector and cleanup
instructions. This is review evidence only; it does not activate runtime
dispatch.

## Required Before SIMD Code Lands

Any AVX2, NEON, AVX-512, wasm `simd128`, or runtime-dispatch implementation
must include:

- Completion of
  [SIMD_ACTIVATION_CHECKLIST.md](SIMD_ACTIVATION_CHECKLIST.md) before the
  backend is wired into dispatch.
- The dedicated `src/simd/` boundary for all architecture-specific code.
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
- The encode admission draft to remain packaged and validated before any
  `1.2.0` dispatch work starts.

When an accelerated backend is ready for admission, update this gate in the
same commit as the scalar differential tests, fuzz evidence, unsafe inventory,
benchmark evidence, and release notes. For encode acceleration, start from
[SIMD_ENCODE_ADMISSION_DRAFT.md](SIMD_ENCODE_ADMISSION_DRAFT.md) and keep any
backend not fully proven in the candidate-only state.
The draft is guarded by `scripts/validate-simd-encode-admission-draft.sh` so
runtime report expectations, benchmark template fields, release-note precision,
and architecture-specific blockers do not drift while `1.1.x` remains
scalar-only.

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
