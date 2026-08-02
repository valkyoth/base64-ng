# SIMD Admission Policy

`base64-ng` is scalar by default and admits conservative accelerated encode
paths in the `1.2.x` line: std `x86`/`x86_64` AVX-512 VBMI first, then
AVX2, then SSSE3/SSE4.1, plus little-endian std `aarch64` NEON, for Standard
and URL-safe alphabet families. Future SIMD dispatch remains gated
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
  CPUs, `no_std`, custom alphabets, in-place encode, line-ending insertion,
  and every decode path. Final tail and padding completion use scalar code.
  Wrapped encode helpers may use
  the admitted backend for their unwrapped staging step when the normal
  `encode_slice` admission conditions are met.
- `1.1.7` admits std `x86`/`x86_64` AVX2 encode dispatch for Standard and
  URL-safe alphabet families. AVX2 is selected before SSSE3/SSE4.1 when runtime
  CPU probing proves `avx2`; otherwise the existing SSSE3/SSE4.1 or scalar
  fallback path is used. Final tail and padding completion use scalar code.
  Custom alphabets, `no_std`, in-place encode, line-ending insertion, and every
  decode path remain scalar. Wrapped encode helpers may use admitted fixed-block encode for their unwrapped staging
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
- `1.2.3` updated the optional `base64-ng-sanitization` companion dependency
  to `sanitization` `1.2.2` and synced workspace package metadata.
- After the `1.2.x` encode release, pause feature work for a short soak period
  so users can report platform-specific encode regressions before decode
  acceleration work starts.
- `1.2.x` is the SIMD decode foundation series. Decode prototypes remain
  non-dispatchable while invalid-input handling, canonicality, padding, output
  retention, error behavior, fuzz coverage, and timing-oriented evidence are
  proven against scalar behavior.
- `1.3.0` is the first release that activates SIMD decode acceleration after
  the decode evidence line completed and the encode acceleration line remained
  stable.
- The admitted `1.3.0` decode backends are std `x86`/`x86_64` AVX-512 VBMI
  first, then AVX2, then SSSE3/SSE4.1 strict decode, plus little-endian std
  `aarch64` NEON strict decode for Standard and URL-safe alphabet families.
  They validate the complete input with the scalar decoder first so public
  error shape and indexes remain scalar-compatible, then use fixed 64-byte
  AVX-512 VBMI, fixed 32-byte AVX2, fixed 16-byte SSSE3/SSE4.1, or fixed
  16-byte NEON encoded blocks where possible. Tails and every unsupported
  decode surface remain scalar.

The `1.3.0` decode scope is frozen to strict
Standard and URL-safe decode only, padded and unpadded, through the normal
strict decode backend boundary. Wrapped decode may use admitted strict decode
after scalar line-profile validation and line-ending compaction. Legacy
whitespace decode may use the admitted strict decode boundary after scalar
whitespace compaction. Strict in-place decode may use admitted strict decode
backends only after stack staging. Custom alphabets, bcrypt-style and
`crypt(3)` profiles, `no_std` SIMD dispatch, broader wasm/browser runtime
dispatch, and the `base64_ng::ct` constant-time-oriented secret decode path
remain scalar unless separately admitted with their own evidence package.

The detailed `1.2.3` to `1.3.0` workflow was commit-based rather than
tag-based. Each planned commit was followed by pentest and CI review before the
next implementation commit started. See
[`docs/PLAN.md`](PLAN.md#commit-based-123-to-130-completion-plan) for the
completed sequence and `1.3.0` acceptance criteria.

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
  boundaries. In-place encode may use admitted encode backends only after
  stack staging protects unread input bytes. Strict decode may use the
  admitted AVX-512 VBMI, AVX2, or SSSE3/SSE4.1 backend on x86/x86_64, or the
  admitted NEON backend on little-endian AArch64, with the `simd` feature.
  `std` uses runtime probing; `no_std` requires complete static target-feature
  evidence and pointer-width atomics. Every unsupported surface falls back to
  scalar.
- With the `simd` feature enabled, the private dispatch scaffold detects
  AVX-512 VBMI, AVX2, SSSE3/SSE4.1, NEON, and wasm `simd128` candidates.
  Admitted x86/x86_64, little-endian AArch64, and narrow wasm backends can
  become active after their direct KAT passes. Normal strict decode uses its
  separate operation-specific health latch. All unsupported candidates still
  execute scalar code.
- Admitted SIMD encode paths run only when the current input can fill at least
  one block. On an AVX-512/VBMI x86 CPU, automatic encode uses SSSE3/SSE4.1 for
  12–23 bytes, AVX2 for 24–191 bytes, and AVX-512 from 192 bytes; exact static and
  evidence APIs may request AVX-512 from its 48-byte block boundary. Other x86
  candidates use their native 24-byte AVX2 or 12-byte SSSE3/SSE4.1 boundary.
  Automatic NEON encode starts at the conservative 192-byte Commit 29
  crossover; exact static/evidence calls retain the 12-byte block boundary.
  Shorter inputs and non-block tails use scalar.
- Public slice, clear-tail, alloc, and wrapped encode helpers route through the
  admitted encode boundary. For wrapped encode, SIMD applies only to the
  unwrapped Base64 staging step; line-ending insertion remains scalar.
- Public strict `decode_slice`, `decode_slice_clear_tail`, `decode_buffer`, and
  alloc strict decode helpers route through the decode boundary. AVX-512 VBMI
  automatic decode applies from the measured 16 KiB encoded-input crossover;
  exact static/evidence calls retain the 64-byte block minimum. Commit 28 maps,
  packs, VBMI-compacts, and masked-stores full AVX-512 blocks directly after
  scalar whole-input validation. Commit 27 does the same for full 32-byte AVX2
  and 16-byte SSSE3/SSE4.1 blocks, with exact-width output stores and one
  register cleanup per call. Commit 29 gives little-endian AArch64 NEON the
  same direct architecture: vector ASCII classification precedes exact 8+4
  byte stores, scalar whole-input validation preserves diagnostics, and vector
  state is cleared once after the full block loop. Automatic NEON strict
  decode begins at 256 encoded bytes; exact static/evidence calls retain the
  16-byte block boundary. Public strict decode
  supports every valid encoded length; short inputs and non-block tails are
  decoded by scalar code. Wrapped decode may use admitted strict decode after
  scalar line-profile validation and line-ending compaction. Legacy whitespace
  decode may use the admitted strict decode boundary after scalar whitespace
  compaction. Strict in-place decode may use admitted strict decode backends
  only after stack staging. CT secret decode, custom alphabets, and big-endian
  AArch64 remain scalar.
- AVX-512 VBMI encode is admitted for `x86`/`x86_64` Standard and URL-safe
  alphabet families. It uses an exact 48-byte masked load, VBMI byte expansion,
  vector shifts/masks, and direct VBMI alphabet lookup for fixed 48-byte input
  blocks, then clears all ZMM state once after the block loop. Runtime
  dispatch uses `std::is_x86_feature_detected!` and requires `avx512f`,
  `avx512bw`, `avx512vl`, and `avx512vbmi`; unsupported CPUs fall back to
  AVX2, SSSE3/SSE4.1, or scalar. Final tail and padding completion use scalar
  code. Custom alphabets, line-ending insertion, and every decode
  surface outside the separate AVX-512/AVX2/SSSE3/SSE4.1/NEON strict decode
  admission stay scalar. In-place encode may enter only through stack staging.
- Runtime backend identifiers expose their required CPU feature bundles through
  `runtime::Backend::required_cpu_features()`.
- Runtime backend reports include `candidate_required_cpu_features=[...]` in
  their stable key/value display output for audit logs.
- Runtime backend reports include `candidate_detection_mode=...` so logs show
  whether a SIMD candidate came from runtime CPU feature probing or from
  compile-time target features.
- Runtime backend reports expose `snapshot()` for structured audit logging
  without parsing formatted strings.
- SSSE3/SSE4.1 encode is admitted for `x86`/`x86_64` Standard and
  URL-safe alphabet families. Commit 25 uses exact 8-byte plus 4-byte reads,
  SSSE3 byte shuffling, SSE lane shifts/masks, and a 16-entry byte-shuffle
  alphabet mapper for fixed 12-byte input blocks. Runtime dispatch uses
  `std::is_x86_feature_detected!`; unsupported CPUs execute scalar code.
  Custom alphabets, final tail/padding completion, line-ending
  insertion, and every decode surface outside the separate
  AVX-512/AVX2/SSSE3/SSE4.1/NEON strict decode admission stay scalar.
  In-place encode may enter admitted encode backends only through stack
  staging.
- AVX2 encode is admitted for `x86`/`x86_64` Standard and URL-safe alphabet
  families. Commit 25 uses exact 16-byte plus 8-byte reads, AVX2 lane-local
  byte shuffling, vector shifts/masks, and a duplicated 16-entry byte-shuffle
  alphabet mapper for fixed 24-byte input blocks. LLVM emits `vzeroupper` at
  the target-feature return boundary after the complete block loop. Runtime
  dispatch uses `std::is_x86_feature_detected!`;
  unsupported CPUs fall back to SSSE3/SSE4.1 or scalar. Final tail and padding
  completion use scalar code. Custom alphabets, line-ending
  insertion, and every decode surface outside the separate
  AVX-512/AVX2/SSSE3/SSE4.1/NEON strict decode admission stay scalar.
  In-place encode may enter admitted encode backends only through stack
  staging.
- Commit 25 classifies these ordinary encode vectors as public-data scratch.
  It removes per-block staging wipes and full XMM/YMM clears from SSSE3/AVX2
  encode without changing the separate scalar secret contract. In
  `no_std + simd`, `StaticBackendToken::encode_standard` and
  `StaticBackendToken::encode_url_safe` expose these kernels only after KAT,
  generation, and quarantine checks. `checked-backend` applies the same bounded
  scalar comparison and quarantine path to these static-token calls as to
  automatic dispatch. Quarantine blocks later admission but does not cancel an
  invocation that already observed a healthy generation.
- Commit 27 rewrites SSSE3/SSE4.1 and AVX2 strict decode as direct production
  kernels. Vector range and equality masks classify Standard or URL-safe ASCII
  and map it to 6-bit values before multiply-add packing; no per-block scalar
  decode, value staging, release-mode scalar comparison, or per-block cleanup
  remains. One whole-input scalar validation still defines exact diagnostics,
  canonicality, required output length, and no-write-on-error behavior. Final
  padding and short tails remain scalar. `StaticBackendToken::decode_standard`
  and `decode_url_safe` expose these kernels to admitted `no_std` deployments;
  `checked-backend` retains redundant scalar comparison and quarantine.
- AArch64 NEON encode is admitted for little-endian `aarch64` Standard and
  URL-safe alphabet families. Commit 29 uses exact 8+4-byte caller-input reads,
  NEON table lookup, vector shifts/masks, and byte-select alphabet mapping for
  fixed 12-byte blocks without stack staging or caller over-read. Strict
  decode directly classifies 16 ASCII lanes, reduces the full validity mask
  before output, compacts four quanta, and stores exactly 8+4 bytes. Production
  loops clear all AArch64 vector registers once after the block sequence.
  Runtime dispatch, `StaticBackendToken`, operation-specific KATs,
  checked-backend comparison, quarantine, and per-operation reports share the
  same kernel boundary. Final tail and padding completion use scalar code.
  Custom alphabets, big-endian AArch64, 32-bit `arm+neon`,
  line-ending insertion, and every decode surface outside the separate
  AVX-512/AVX2/SSSE3/SSE4.1/NEON strict decode admission stay scalar. In-place
  encode may enter only through stack staging.
- The non-standard encode surface review keeps alphabet and line-wrapping
  claims narrow. In-place encode enters admitted encode backends only through
  stack staging so overlapping output never overwrites unread input. Bcrypt,
  `crypt(3)`, custom alphabets, and other non-Standard-family alphabets remain
  scalar because accelerated alphabet mapping has not been separately proven.
  Wrapped encode may still use the admitted unwrapped staging step, but
  line-ending insertion is scalar. `no_std` has no runtime probing; it may use
  SIMD only when the complete compile-time target-feature bundle and atomic
  backend-health latch are available, otherwise it remains scalar.
- 2.0 Commit 30 rebuilds wasm `simd128` as direct production fixed-block
  encode and strict decode for Standard and URL-safe alphabet families.
  Encode loads exactly 12 bytes and stores exactly 16 bytes. Decode performs
  whole-input scalar validation once, directly classifies each 16-byte vector,
  reduces all validity lanes before exact 12-byte stores, and leaves padded
  final quanta and tails to scalar code. There is no per-block scalar
  comparison in the admitted hot loop. Scalar and SIMD decoding both derive
  mappings directly from `Alphabet::ENCODE`; overridable `Alphabet::decode`
  code is never executed as an admission check or an engine mapping boundary.
  Non-Standard-family tables remain scalar unless separately admitted.
  The supported `base64-ng-wasm-loader` npm package ships separate scalar and
  SIMD artifacts and selects with an embedded `WebAssembly.validate` probe
  before instantiation. Exact-package Node/V8, Wasmtime, Chromium/V8,
  Firefox/SpiderMonkey, and operator-run Safari/WebKit evidence covers codec
  sweeps, malformed input, hostile JavaScript values, transactionality,
  ceilings, package contents, and disposal. This is not a universal JIT timing,
  register-retention, or cleanup guarantee. The release-facing decision is
  tracked in
  [WASM_SIMD128_RUNTIME_REVIEW.md](WASM_SIMD128_RUNTIME_REVIEW.md).
- Big-endian and RISC-V acceleration work follows a QEMU-first evidence path.
  SVE candidate work follows the same QEMU-first evidence discipline.
  QEMU proves functional behavior, not hardware performance, timing,
  microarchitectural, register-retention, signal-state, or side-channel
  properties. Commit 32 provides a vector-length-independent RVV 1.0
  Standard/URL-safe encode and strict-decode candidate behind the internal
  `base64_ng_rvv_candidate` evidence cfg. It is exercised at VLEN 128 and 256,
  has generated instruction/register-cleanup evidence, and uses fail-closed
  Linux `riscv_hwprobe`, auxiliary-vector, and vector-state checks. Normal
  published builds remain scalar on RISC-V until the native hardware contract
  and external RISC-V vector review pass.
  Commit 33 similarly provides a vector-length-independent AArch64 SVE
  Standard/URL-safe encode and strict-decode candidate behind the internal
  `base64_ng_sve_candidate` evidence cfg. It uses four active lanes at every
  legal vector length, is exercised under QEMU at 128, 256, and 512 bits, has
  generated leaf-assembly and register-cleanup evidence, and fails closed on
  missing HWCAP or invalid per-thread `PR_SVE_GET_VL` results. Normal public
  AArch64 dispatch remains admitted NEON or scalar until evidence from at
  least two real SVE systems with different vector lengths and external review
  satisfy the native admission contract.
- `runtime::backend_report()` reports the active backend, detected candidate,
  detection mode, SIMD feature status, security posture, and a
  conservative unsafe-boundary posture flag. The flag is true only when the
  `simd` feature is disabled; SIMD-enabled builds include additional private
  audited unsafe boundaries and must use the release evidence scripts for
  boundary validation. Commit 24 adds per-operation health state, generation,
  and stable backend-fault telemetry.
- On `x86`/`x86_64` with `std`, candidate detection uses
  `std::is_x86_feature_detected!` runtime CPU probing. On `no_std`, wasm, and
  current ARM builds, candidate detection is compile-time target-feature
  reporting. A binary compiled with `-C target-feature=+avx2` can therefore
  report an AVX2 candidate even if it is deployed on a CPU that cannot execute
  AVX2 instructions. Safe `no_std` acceleration therefore requires the full
  compile-time target-feature bundle, pointer-width atomics, and a passing
  direct KAT. The thread-bound `StaticBackendToken::assume_supported` is the
  only unsafe deployment-attested alternative; false attestation violates its
  safety contract.
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
  AVX2, SSSE3/SSE4.1, little-endian std aarch64 NEON, and narrow wasm
  `simd128` encode paths where the platform requirements are met.
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

This does not execute native accelerated code. It proves the reserved AVX2,
AVX-512, SSSE3/SSE4.1, NEON, and wasm `simd128` feature-gated code still
compiles under `no_std` when the corresponding Rust targets are installed. For
wasm `simd128`, it also builds the wasm test binaries with `simd128` enabled so
the admitted fixed-block wasm code is checked; runtime execution is covered by
`scripts/check_wasm_runtime_dispatch.sh` when Node/V8 and Wasmtime are
installed, by `scripts/check_wasm_browser_dispatch.sh` when a Chromium-family
browser is installed, by `scripts/check_wasm_browser_firefox_dispatch.sh`
when Firefox plus `geckodriver` are installed, and by
`scripts/check_wasm_browser_safari_dispatch.sh` on macOS with Safari remote
automation enabled.

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
admitted strict decode status labels for AVX-512 VBMI, AVX2, SSSE3/SSE4.1,
NEON, and wasm `simd128`, and
`active_backend_admitted=avx512-vbmi-or-avx2-or-ssse3-sse4.1-or-neon-or-wasm-simd128-encode`
for admitted encode backends. The runtime report also exposes
`BackendReport::active_decode_backend()` so release evidence can distinguish
the narrower AVX-512/AVX2/SSSE3/SSE4.1/NEON/wasm strict decode admission from
the active encode backend.

Capture generated assembly evidence for x86 encode paths:

```sh
scripts/generate_simd_asm_evidence.sh
```

The script emits release test-harness assembly for the admitted AVX-512 VBMI,
AVX2, and SSSE3/SSE4.1 encode/decode paths, then checks for expected vector and
cleanup instructions. When the `aarch64-unknown-linux-gnu` target is installed,
it also emits AArch64 NEON assembly evidence and checks table lookup,
bit-select, decode packing, and cleanup instructions. Cross-host runs record
NEON library assembly and compile evidence; real AArch64 hosts must also run
`scripts/check_aarch64_linux.sh` or `scripts/check_macos.sh` for test-harness
execution evidence.

Commit 29 also provides a focused cross-host AArch64 evidence gate:

```sh
scripts/check-2.0-neon-hot-paths.sh
```

On little-endian AArch64 this executes exhaustive direct-kernel tests, static
`no_std` and checked-static tokens, fuzz-build contracts, and optional exact
performance evidence. On other hosts it cross-checks AArch64 compilation,
Clippy, direct-kernel source invariants, and generated release assembly. Set
`BASE64_NG_RUN_COMMIT29_PERF=1` on both an Apple Silicon Mac and a server-class
AArch64 Linux host to produce and validate the required hardware campaign.

## Required Before SIMD Code Lands

Any broader wasm `simd128` runtime/browser profile, additional decode backend,
custom alphabet, in-place extension, or additional runtime-dispatch
implementation must include the surface ledger in
[SIMD_NON_STANDARD_SURFACE_REVIEW.md](SIMD_NON_STANDARD_SURFACE_REVIEW.md) and
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
  VBMI, AVX2, SSSE3/SSE4.1, little-endian std aarch64 NEON, and narrow wasm
  `simd128` encode variants.
- `active_backend()` to return AVX-512 VBMI before AVX2 before SSSE3/SSE4.1
  only after std runtime CPU probing, and scalar otherwise.
- No generic SIMD dispatch variants in source.
- `docs/SIMD_ADMISSION.md` to record the admitted AVX-512 VBMI, AVX2,
  SSSE3/SSE4.1, NEON, and wasm `simd128` encode scope and keep all other
  backends prototype-only.
- Documentation for benchmark evidence, release-note restrictions, and
  vector-register retention cleanup strategy to remain packaged.
- The encode admission draft to remain packaged and validated before any future
  encode dispatch scope expands beyond the currently admitted native and narrow
  wasm backends.

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
  little-endian aarch64 NEON encode for Standard and URL-safe alphabet
  families. The
  `1.3.0` decode admission is separate: std x86/x86_64 AVX-512 VBMI first,
  then AVX2, then SSSE3/SSE4.1, plus little-endian std aarch64 NEON strict
  decode only.
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
