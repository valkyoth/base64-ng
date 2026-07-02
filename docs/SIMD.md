# SIMD Admission Policy

`base64-ng` is scalar by default and admits conservative accelerated encode
paths in the `1.2.x` line: std `x86`/`x86_64` AVX-512 VBMI first, then
AVX2, then SSSE3/SSE4.1, plus std `aarch64` NEON, for Standard and URL-safe
alphabet families. Future SIMD dispatch remains gated
unless a complete SIMD admission evidence package lands in the same release
commit as the active backend change. The crate uses `#![deny(unsafe_code)]` and permits
reviewed `allow(unsafe_code)` exceptions only for audited cleanup in
`src/cleanup.rs`, CT comparison, byte accumulation, CT scan, and CT result-gate
helpers in `src/ct/`, and the private `src/simd/` boundary.

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
  and release evidence. GitHub checkpoint tags in this line moved evidence
  forward without a matching crates.io publish until the `1.2.0` family sync.
- `1.1.5` adds the public encode backend boundary while still forcing scalar
  execution. This gives future accelerated encode admission one reviewed
  integration point for `encode_slice`, clear-tail helpers, alloc helpers,
  wrapped helpers, and in-place encode. The same checkpoint also adds a
  scalar-forced decode backend boundary for symmetry; decode acceleration
  remains out of scope until the later decode line.
- `1.1.6` admits std `x86`/`x86_64` SSSE3/SSE4.1 encode dispatch for Standard
  and URL-safe alphabet families. It processes fixed 12-byte blocks with vector
  code after runtime CPU probing. Scalar remains the fallback for unsupported
  CPUs, `no_std`, custom alphabets, tails, padding, in-place encode,
  line-ending insertion, and every decode path. Wrapped encode helpers may use
  the admitted backend for their unwrapped staging step when the normal
  `encode_slice` admission conditions are met.
- `1.1.7` admits std `x86`/`x86_64` AVX2 encode dispatch for Standard and
  URL-safe alphabet families. AVX2 is selected before SSSE3/SSE4.1 when runtime
  CPU probing proves `avx2`; otherwise the existing SSSE3/SSE4.1 or scalar
  fallback path is used. Tails, padding, custom alphabets, `no_std`, in-place
  encode, line-ending insertion, and every decode path remain scalar. Wrapped
  encode helpers may use admitted fixed-block encode for their unwrapped staging
  step.
- `1.2.0` is the release where encode acceleration became fully working for
  the admitted encode scope. Public encode APIs dispatch to admitted
  AVX-512 VBMI, AVX2, SSSE3/SSE4.1, or NEON encode backends when runtime policy
  and CPU features allow it, and fall back to scalar for unsupported CPUs,
  `no_std`, custom alphabets unless separately admitted, in-place encode,
  line-ending insertion, legacy profiles, tails, and padding. Wrapped encode
  helpers may use admitted SIMD for the unwrapped staging step when they route
  through `encode_slice`. Backends without complete evidence remain real
  non-dispatchable prototypes.
- `1.2.1` is a documentation/package patch for the released `1.2.0` encode
  acceleration scope. It does not admit additional backends.
- `1.2.2` is an encode ergonomics and sanitization hardening patch that adds
  explicit infallible encode convenience helpers and tightens fixed-size locked
  secret decode cleanup. It does not admit additional backends.
- `1.2.3` updates the optional `base64-ng-sanitization` companion dependency to
  `sanitization` `1.2.2` and syncs workspace package metadata. It does not
  admit additional backends.
- After the `1.2.x` encode release, pause feature work for a short soak period
  so users can report platform-specific encode regressions before decode
  acceleration work starts.
- `1.2.x` is the SIMD decode foundation series. Decode prototypes remain
  non-dispatchable while invalid-input handling, canonicality, padding, output
  retention, error behavior, fuzz coverage, and timing-oriented evidence are
  proven against scalar behavior.
- `1.3.0` is the first release that may activate SIMD decode acceleration if
  the `1.2.x` decode evidence line is complete and the encode acceleration line
  has remained stable.
- The admitted `1.3.0` decode backends are std `x86`/`x86_64` AVX2 first,
  then SSSE3/SSE4.1 strict decode for Standard and URL-safe alphabet families.
  They validate the complete input with the scalar decoder first so public
  error shape and indexes remain scalar-compatible, then use fixed 32-byte
  AVX2 or fixed 16-byte SSSE3/SSE4.1 encoded blocks where possible. Tails and
  every unsupported decode surface remain scalar.

The `1.3.0` decode scope is frozen before implementation starts: strict
Standard and URL-safe decode only, padded and unpadded, through the normal
strict decode backend boundary. Wrapped decode, legacy whitespace decode,
custom alphabets, bcrypt-style and `crypt(3)` profiles, in-place decode,
`no_std` SIMD dispatch, wasm runtime dispatch, and the `base64_ng::ct`
constant-time-oriented secret decode path remain scalar unless separately
admitted with their own evidence package.

The detailed `1.2.3` to `1.3.0` workflow is commit-based rather than
tag-based. Each planned commit is followed by pentest and CI review before the
next implementation commit starts. See
[`docs/PLAN.md`](PLAN.md#commit-based-123-to-130-completion-plan) for the
complete sequence and `1.3.0` acceptance criteria.

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
- Encode and normal strict decode entry points pass through internal backend
  boundaries. In-place encode remains scalar-only. Strict decode may use the
  admitted AVX2 or SSSE3/SSE4.1 backend on std x86/x86_64 builds with the
  `simd` feature; every unsupported decode surface still falls back to scalar.
- With the `simd` feature enabled, the private dispatch scaffold detects
  AVX-512 VBMI, AVX2, SSSE3/SSE4.1, NEON, and wasm `simd128` candidates.
  Only std `x86`/`x86_64` AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and std
  `aarch64` NEON encode can become active; all other candidates still execute
  scalar code.
- Admitted SIMD encode paths run only when the current input can fill at least
  one block for the selected backend: 48 bytes for AVX-512 VBMI, 24 bytes for
  AVX2, and 12 bytes for SSSE3/SSE4.1 or NEON. Shorter inputs use scalar encode
  before SIMD dispatch, and non-block tails remain scalar.
- Public slice, clear-tail, alloc, and wrapped encode helpers route through the
  admitted encode boundary. For wrapped encode, SIMD applies only to the
  unwrapped Base64 staging step; line-ending insertion remains scalar.
- Public strict `decode_slice`, `decode_slice_clear_tail`, `decode_buffer`, and
  alloc strict decode helpers route through the decode boundary. AVX2 decode
  applies only to full 32-byte encoded blocks after scalar whole-input
  validation and falls back to SSSE3/SSE4.1 or scalar for shorter inputs.
  Legacy decode, wrapped decode, in-place decode, CT secret decode, custom
  alphabets, short inputs, and tails remain scalar.
- AVX-512 VBMI encode is admitted for std `x86`/`x86_64` Standard and URL-safe
  alphabet families. It uses AVX-512 lane-local byte shuffling, vector
  shifts/masks, and VBMI byte permutes over the alphabet table for fixed
  48-byte input blocks, then clears ZMM/YMM state before returning. Runtime
  dispatch uses `std::is_x86_feature_detected!` and requires `avx512f`,
  `avx512bw`, `avx512vl`, and `avx512vbmi`; unsupported CPUs fall back to
  AVX2, SSSE3/SSE4.1, or scalar. Custom alphabets, tails, padding, `no_std`,
  in-place encode, line-ending insertion, and every decode surface outside the
  separate AVX2/SSSE3/SSE4.1 strict decode admission stay scalar.
- Runtime backend identifiers expose their required CPU feature bundles through
  `runtime::Backend::required_cpu_features()`.
- Runtime backend reports include `candidate_required_cpu_features=[...]` in
  their stable key/value display output for audit logs.
- Runtime backend reports include `candidate_detection_mode=...` so logs show
  whether a SIMD candidate came from runtime CPU feature probing or from
  compile-time target features.
- Runtime backend reports expose `snapshot()` for structured audit logging
  without parsing formatted strings.
- SSSE3/SSE4.1 encode is admitted for std `x86`/`x86_64` Standard and
  URL-safe alphabet families. It uses SSSE3 byte shuffling, SSE lane
  shifts/masks, and SSE4.1 byte blending for fixed 12-byte input blocks, then
  clears XMM registers before returning. Runtime dispatch uses
  `std::is_x86_feature_detected!`; unsupported CPUs execute scalar code.
  Custom alphabets, tails, padding, `no_std`, in-place encode, line-ending
  insertion, and every decode surface outside the separate AVX2/SSSE3/SSE4.1
  strict decode admission stay scalar.
- AVX2 encode is admitted for std `x86`/`x86_64` Standard and URL-safe alphabet
  families. It uses AVX2 lane-local byte shuffling, vector shifts/masks, and
  byte blending for fixed 24-byte input blocks, then clears XMM/YMM state
  before returning. Runtime dispatch uses `std::is_x86_feature_detected!`;
  unsupported CPUs fall back to SSSE3/SSE4.1 or scalar. Custom alphabets, tails,
  padding, `no_std`, in-place encode, line-ending insertion, and every decode
  surface outside the separate AVX2/SSSE3/SSE4.1 strict decode admission stay
  scalar.
- AArch64 NEON encode is admitted for std `aarch64` Standard and URL-safe
  alphabet families. It uses NEON table lookup, vector shifts/masks, and
  byte-select alphabet mapping for fixed 12-byte input blocks, then clears used
  NEON registers before returning. NEON is mandatory for the admitted AArch64
  target. Custom alphabets, tails, padding, 32-bit `arm+neon`, `no_std`,
  in-place encode, line-ending insertion, and every decode surface outside the
  separate AVX2/SSSE3/SSE4.1 strict decode admission stay scalar.
- An inactive wasm `simd128` fixed-block encode prototype exists behind the
  same boundary as real non-dispatchable vector encode evidence for Standard
  and URL-safe alphabets. It uses wasm byte shuffling, vector shifts/masks, and
  branchless Standard-family alphabet mapping for fixed 12-byte input blocks.
  Custom alphabets remain scalar scaffold paths because portable wasm SIMD does
  not provide a direct 64-byte alphabet lookup. The wasm feature-bundle check
  builds wasm test binaries with `target-feature=+simd128`; this is compile and
  codegen evidence only, not a runtime/JIT timing or register-retention claim.
  Runtime backend selection remains scalar for wasm.
- `runtime::backend_report()` reports the active backend, detected candidate,
  detection mode, SIMD feature status, security posture, and a
  conservative unsafe-boundary posture flag. The flag is true only when the
  reserved `simd` feature is disabled; SIMD-enabled builds include additional
  private prototype boundaries and must use the release evidence scripts for
  boundary validation.
- On `x86`/`x86_64` with `std`, candidate detection uses
  `std::is_x86_feature_detected!` runtime CPU probing. On `no_std`, wasm, and
  current ARM builds, candidate detection is compile-time target-feature
  reporting. A binary compiled with `-C target-feature=+avx2` can therefore
  report an AVX2 candidate even if it is deployed on a CPU that cannot execute
  AVX2 instructions. Active x86/x86_64 encode dispatch is std runtime-probed
  only; active AArch64 NEON encode dispatch is std-only and relies on the
  mandatory AArch64 NEON target contract. Any future `no_std` SIMD activation
  must require an explicit caller-side CPU contract or remain disabled where
  runtime probing is unavailable.
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
- The `simd` feature enables only the admitted std x86/x86_64 AVX-512 VBMI,
  AVX2, SSSE3/SSE4.1, and std aarch64 NEON encode paths where the platform
  requirements are met.
- Current `1.2.x` development keeps every non-admitted backend scalar or
  prototype-only unless the SIMD admission manifest, scalar differential tests,
  fuzz evidence, unsafe inventory, architecture evidence, benchmark evidence,
  and release wording are updated together.
- Decode acceleration is higher risk than encode acceleration because the
  accelerated path must match scalar behavior for invalid bytes, padding
  placement, non-canonical trailing bits, undersized outputs, partial-output
  cleanup, and public error behavior. No decode backend may dispatch until
  those properties are covered by tests, fuzz evidence, generated-code review,
  unsafe inventory, hardware evidence where applicable, and release wording.
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

This prints the runtime backend-report test and runs the gated SIMD
scalar-equivalence tests with `--nocapture`, so local CPU evidence is easy to
copy into release notes or issue discussion. On x86/x86_64 hosts with AVX-512
VBMI, AVX2, or SSSE3/SSE4.1, and on aarch64 hosts with NEON, the runtime report
may show admitted encode acceleration as active. On 32-bit ARM, NEON remains
scaffold evidence. The script also writes
`target/release-evidence/backend/MANIFEST.txt` with toolchain metadata,
commands, status values, artifact checksums, and explicit
`prototype_state=real-non-dispatchable` labels for prototype-only backends,
including the non-dispatchable AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and NEON
fixed-block decode prototypes, and
`active_backend_admitted=avx512-vbmi-or-avx2-or-ssse3-sse4.1-or-neon-encode`
for admitted encode backends. The runtime report also exposes
`BackendReport::active_decode_backend()` so release evidence can distinguish
the narrower AVX2/SSSE3/SSE4.1 strict decode admission from the active encode
backend.

Capture generated assembly evidence for x86 encode paths:

```sh
scripts/generate_simd_asm_evidence.sh
```

The script emits release test-harness assembly for the admitted AVX-512 VBMI,
AVX2, and SSSE3/SSE4.1 encode paths, then checks for expected vector and
cleanup instructions. When the `aarch64-unknown-linux-gnu` target is installed,
it also emits AArch64 NEON assembly evidence and checks table lookup,
bit-select, and cleanup instructions. On cross-host runs this covers admitted
NEON encode library assembly; NEON decode test-harness assembly evidence is
generated on real AArch64 hosts.

## Required Before SIMD Code Lands

Any wasm `simd128`, additional decode backend, custom alphabet, in-place, or
additional runtime-dispatch implementation
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

`scripts/validate-simd-admission.sh` keeps SIMD dispatch limited to admitted
backends. The gate currently requires:

- `ActiveBackend` to expose only `Scalar` plus the std x86/x86_64 AVX-512
  VBMI, AVX2, SSSE3/SSE4.1, and std aarch64 NEON encode variants.
- `active_backend()` to return AVX-512 VBMI before AVX2 before SSSE3/SSE4.1
  only after std runtime CPU probing, and scalar otherwise.
- No accelerated `ActiveBackend::Wasm*` or generic SIMD dispatch variants in
  source.
- `docs/SIMD_ADMISSION.md` to record the admitted AVX-512 VBMI, AVX2,
  SSSE3/SSE4.1, and NEON encode scope and keep all other backends
  prototype-only.
- Documentation for benchmark evidence, release-note restrictions, and
  vector-register retention cleanup strategy to remain packaged.
- The encode admission draft to remain packaged and validated before any future
  encode dispatch scope expands beyond the currently admitted `1.2.x` backends.

When an accelerated backend is ready for admission, update this gate in the
same commit as the scalar differential tests, fuzz evidence, unsafe inventory,
benchmark evidence, and release notes. For encode acceleration, start from
[SIMD_ENCODE_ADMISSION_DRAFT.md](SIMD_ENCODE_ADMISSION_DRAFT.md) and keep any
backend not fully proven in the candidate-only state.
The draft is guarded by `scripts/validate-simd-encode-admission-draft.sh` so
runtime report expectations, benchmark template fields, release-note precision,
and architecture-specific blockers do not drift while later encode backends
remain pending.

## Dispatch Rules

- Scalar remains the fallback for every build.
- Candidate detection must not imply activation; a detected candidate may still
  execute scalar until the accelerated backend is admitted.
- The active non-scalar backends in the `1.2.x` encode line are std
  x86/x86_64 AVX-512 VBMI encode, AVX2 encode, SSSE3/SSE4.1 encode, and std
  aarch64 NEON encode for Standard and URL-safe alphabet families. The
  `1.3.0` decode admission is separate: std x86/x86_64 AVX2 first, then
  SSSE3/SSE4.1 strict decode only.
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
