# SIMD Encode Admission Draft

This draft is the reusable working package for any future encode acceleration
scope beyond the `1.2.0` release. It is not an admission record. AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and NEON encode are already admitted for std x86/x86_64 or std aarch64 Standard and URL-safe alphabet families; every additional backend or broader API surface remains pending until `docs/SIMD_ADMISSION.md`, release evidence, tests, benchmarks, unsafe inventory, and release notes are updated in the same commit as the active backend change.

## Scope

The first admission candidate should be encode-only. Decode acceleration remains
out of scope because malformed input, canonicality, padding, output retention,
and timing behavior make decode higher risk than encode.

The conservative initial activation shape is:

- `std` x86/x86_64 and std aarch64 dispatch only. x86/x86_64 uses
  `std::is_x86_feature_detected!` runtime CPU feature probing, while aarch64
  NEON relies on the mandatory target contract.
- Release gate phrase: std x86/x86_64 and std aarch64 dispatch only.
- `no_std` builds remain scalar-only unless a later unsafe caller-contract API
  is designed and reviewed.
- Unsupported CPUs must execute scalar code without illegal instructions.
- Scalar remains the reference behavior for all profiles, tails, padding, and
  custom alphabets.
- Any backend whose evidence is incomplete stays candidate-only.

## Candidate Backend Requirements

Each encode backend considered for admission must provide:

- fixed-block scalar equivalence for Standard and URL-safe alphabets
- full `Engine::encode_slice`, `encode_slice_clear_tail`, and alloc helper
  coverage with scalar tail handling
- undersized-output error parity with scalar behavior
- in-place encode parity where the backend is used
- custom alphabet fallback behavior, or a separately proven custom-alphabet
  vector path
- fuzz differential evidence against the scalar implementation
- generated optimized assembly showing expected vector instructions and
  register cleanup
- hardware evidence from a CPU that actually supports the backend
- benchmark output with scalar baseline on the same machine

Architecture-specific admission blockers:

- AArch64 NEON must include generated assembly that identifies every
  caller-derived vector register and any callee-saved vector spill/restore slot
  that may carry caller data.
- wasm `simd128` must include generated-code/JIT evidence for the selected wasm
  runtime, or the release must explicitly keep wasm SIMD candidate-only and make
  no runtime register-retention claim.

## Runtime Report Expectations

Before active encode dispatch can ship, runtime reporting must make the
admission visible and auditable:

- `active` must name the admitted backend when that backend is selected.
- `candidate` must continue to name the strongest visible backend candidate.
- `candidate_required_cpu_features` must list the exact feature bundle.
- `accelerated_backend_active=true` only when dispatch uses an admitted backend.
- `security_posture=accelerated` only when an accelerated backend is active.
- `candidate_detection_mode` must distinguish runtime probing from compile-time
  target-feature reporting.
- `unsafe_boundary_enforced` must remain conservative for `simd` builds and be
  explained in release notes.

Policy tests must prove:

- scalar-only builds continue to satisfy `ScalarExecutionOnly`
- unsupported runtime CPUs fall back to scalar
- SIMD-enabled builds that do not activate acceleration report candidate-only
  posture
- high-assurance scalar policy still rejects builds with the reserved `simd`
  feature enabled

## Benchmark Template

Release notes may cite performance only with a complete benchmark record:

```text
backend:
target triple:
CPU model:
OS/kernel:
Rust version:
command:
input sizes:
scalar throughput:
SIMD throughput:
speedup:
raw artifact:
```

Minimum command set for a release candidate:

```sh
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
scripts/check_simd_feature_bundles.sh
scripts/check_backend_evidence.sh
scripts/generate_simd_asm_evidence.sh
BASE64_NG_RUN_PERF=1 scripts/check_perf.sh
```

## Release Note Template

Allowed wording:

```text
This release admits std-only x86_64 AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and
std-only aarch64 NEON encode dispatch for Standard and URL-safe Base64 on
platforms that pass the admission checks. Unsupported CPUs continue to use the
scalar backend. Decode remains scalar-only.
```

Required precision:

- name the active backend or backends
- name backends that remain candidate-only
- say whether dispatch is `std`-only
- say that `no_std` remains scalar unless an explicit caller contract exists
- cite benchmark hardware and commands for any speed claim
- keep security caveats separate from performance claims

Forbidden wording until proven:

- "SIMD acceleration" without naming which backend is active
- portable throughput numbers without hardware and command context
- claims that SIMD decode is active when only encode is admitted
- claims that wasm runtime/JIT timing or register-retention behavior is proven

## Admission Decision Checklist

Before changing `ActiveBackend`, answer all of these in the release PR:

- Which exact backend is admitted?
- Which public APIs call it?
- Which public APIs remain scalar-only?
- How are tails handled?
- How are custom alphabets handled?
- How does unsupported hardware fall back without illegal instructions?
- Which tests prove scalar equivalence?
- Which fuzz targets were run and for how long?
- Which assembly artifacts show register cleanup?
- Which benchmark artifacts support release-note claims?
- Which docs changed in the same commit?
