# Ordinary Decode Validation

2.1 adds the non-exhaustive `DecodeValidation::{Auto, ScalarReference}` policy.
It is a per-call choice, not a Cargo feature or a change to the codec grammar.
`Auto` uses health-gated SSSE3/SSE4.1, AVX2, little-endian AArch64 NEON, or WASM simd128
validation and writing for strict
Standard/URL-safe padded/unpadded presets and exactly equivalent runtime
settings. Canonical fallback uses portable table validation; `ScalarReference`
retains the original validator. Eligible historical x86/NEON/WASM calls share the fast
core without acquiring canonical error semantics. Automatic AVX-512 remains
outside the current production route.

The new vector route starts at 512 encoded bytes on x86/WASM and 4096 on AArch64;
smaller canonical calls keep
portable validation/writing and smaller historical calls keep their existing
decoder. This conservative complete-call cutoff avoids measured small-message
setup regressions on the measured native hosts; WASM uses the conservative
512-byte integration floor. It does not change existing static/exact ISA contracts.
WASM requires a simd128-compiled artifact; this is not native CPU detection.
The scalar artifact never selects this path, and host JIT behavior is outside
Rust's code-generation guarantees.

```rust
use base64_ng::{DecodeValidation, STRICT_STANDARD_PADDED};
let policy = DecodeValidation::ScalarReference;
let mut output = [0; 3];
let written = STRICT_STANDARD_PADDED
    .decode_into_with_validation(b"Zm9v", &mut output, policy)?;
assert_eq!(&output[..written], b"foo");
# Ok::<(), base64_ng::OneShotError>(())
```

## Surfaces

| Surface | Explicit method |
| --- | --- |
| Canonical caller buffer | `Base64::decode_into_with_validation` |
| Canonical caller buffer with execution report | `Base64::decode_into_with_report` |
| Transactional static token with execution report | `decode_standard_with_report`, `decode_url_safe_with_report` |
| Canonical validation and exact length | `validate_with_validation`, `decoded_len_with_validation` |
| Canonical allocating helpers | `decode_to_vec_with_validation`, `decode_to_vec_with_limit_and_validation` |
| Borrowed validated input | `Base64Ref::parse_with_validation`, then `decode_into` or allocating helpers |
| Ordinary incremental input accepted by one call | `DecoderState::update_with_validation` |
| Historical caller buffer | `Engine::decode_slice_with_validation` |
| Historical clearing caller buffer | `decode_slice_clear_tail_with_validation` |
| Historical validation and fully validated length | `validate_result_with_validation`, `validated_decoded_len_with_validation` |
| Historical allocating helper | `decode_vec_with_validation` |

All existing signatures remain available. The original `Engine::decoded_len`
still checks shape rather than full validity; the new fully validated length
method deliberately has a different name. Runtime/custom canonical codecs use
their exact existing padding and tail-bit settings. This option does not make a
compatibility codec strict or turn a strict codec into a forgiving one.

Canonical decoding stays transactional: validation errors precede capacity or
allocation-limit errors, and any returned error leaves the destination unchanged.
Historical decoding keeps its existing error precedence and mutation behavior,
including partial output on its scalar error path. No progressive API is added.

## Composition

Per-call reports and the shared validation/decode health latch are described in
[the Commit 15 checkpoint](DECODE_COMPOSITION_2.1.md). A capability snapshot is
not evidence that a particular invocation actually executed a vector backend.

- `ScalarReference` means the original scalar grammar checks, not scalar output instructions.
  Valid input is fully checked. Malformed input can fail early; this is not CT.
  The existing scalar decoder combines validation with output generation;
  reference-selected historical SIMD decoders retain scalar prevalidation. This preserves historical behavior
  without adding a redundant extra pass to every scalar decode.
- Canonical `Auto` checks complete grammar with admitted vector blocks and
  portable input-indexed tail tables (or entirely portable tables),
  then writes through the private preflight result. On rejection, the original
  validator recovers exact diagnostics before capacity or allocation checks.
  A late-invalid input can therefore take two linear scans, still `O(n)`.
  Network-facing callers should enforce protocol-level input-size limits;
  detailed ordinary diagnostics and these tables are not constant-time.
  Custom alphabets and relaxed settings retain the original validator and
  writer. Empty input is accepted without inspecting the alphabet.
- Canonical writing uses admitted vector blocks and specialized tail tables
  under either validation policy when settings qualify. Reference validation still runs the original
  incremental state machine, not the optimized table validator.
- Historical `Auto` shares the fast core for eligible x86/NEON/WASM calls with sufficient
  capacity. Invalid input, insufficient capacity, custom alphabets and other
  decode backends use the original path, preserving historical errors and
  partial-write behavior. Fully validated length helpers also share fast
  validation. The explicit reference choice remains available.
  Historical error recovery can run both canonical reference diagnostics and
  the historical decoder after fast rejection, so rejection remains linear
  but may involve three scans. Enforce application-level input-size limits.
- `checked-backend` still performs its independent output comparison for selected
  accelerated backends and retains quarantine/retry rules. Neither policy opts out.
- Scalar-only builds require neither allocation nor CPU detection. The policy
  works in `no_std`; only allocating helpers require `alloc`.
- Static ISA tokens keep their existing contracts; this option neither creates
  tokens nor overrides execution/deployment restrictions.
- Incremental `update_with_validation` applies to newly accepted input, not
  previously pending output. Complete accepted-prefix validation precedes all
  writes in that call, but cannot roll back earlier successful calls. Finish
  retains scalar terminal rules. See [incremental bulk processing](INCREMENTAL_BULK_2.1.md).
- CT engines and secret frames do not accept this policy or acquire it through
  an implicit conversion. Continue using the separate secret APIs for secrets.

## Verification

Run `sh scripts/check-2.1-validation-policy.sh`. Tests exercise ordinary public
APIs from a downstream integration target, const policy/specification values,
generic codecs, runtime alphabets, malformed-input precedence, whole-buffer
sentinels, allocation limits, feature combinations and Rust 1.90.0. Internal
observations distinguish the original validator from the specialized validator.
Tests exhaust both 256-byte classifications and all two/three-sextet tail
combinations, check custom-alphabet fallback, and compare errors and whole
destinations across malformed positions and capacities under both policies.
The existing 2.0.4 downstream fixture remains unchanged.
See [Commit 5 measurements](PERFORMANCE_2.1_SCALAR.md) for the bounded local
development comparison, its scope and limitations.

## Private Preflight Boundary

Commit 4 binds successful ordinary validation to an immutable input borrow and
an owned codec-settings snapshot. The private result is neither `Clone` nor
`Copy`; the writer consumes it and accepts no replacement source or settings.
Commit 16 adds an internal reborrow operation restricted to the same source,
settings and layout. The [borrowed-view owner](BORROWED_BASE64_2.1.md) checks
the retained validation policy and validator health before each reborrow;
explicit reference validation and stale validator generations require a fresh
reference proof. This does not cache permission to execute a backend.
Canonical caller-buffer, allocating and borrowed-view decode share this
boundary, as do eligible historical ordinary decoding, static-token decoding
and bulk incremental decoding. Allocating decode retains the result across
reservation instead of validating again; incremental decode binds a proof to
the bulk portion accepted by one update, not the entire stream. Historical
validation-only helpers share its length checks. CT/secret paths remain
separate and are not rerouted.

Commit 19 adds [ordinary in-place compaction](IN_PLACE_BULK_2.1.md). It borrows
the exclusive buffer for complete preflight, then privately retains its settings
and checked length. The immutable proof is not reused across mutation. Each
source chunk is copied before its overlapping destination is borrowed, allowing
the existing disjoint writer and scalar recovery to operate on preserved input.

Checked geometry reserves the last quantum (at most four input bytes and three
output bytes), leaving only complete unpadded quanta in the interior. Empty
input, impossible lengths, arithmetic bounds and the measured output length
are checked before the destination is sliced. The geometry check is not a
grammar validator: a complete surface-specific validator must succeed first.
Commit 5 adds a reviewed portable validator for the exact canonical settings;
custom alphabets, relaxed settings and explicit `ScalarReference` keep the
original validation. The fast validator also verifies padding placement and
canonical tail bits; it is not merely an interior-block classifier.

The classifier/reference disagreement contract is covered with test injection:

| Classifier vs reference | Outcome before any write |
| --- | --- |
| Both reject | Retain the reference's exact surface-specific diagnostic |
| Both accept | Require checked span bounds and destination capacity |
| Either disagrees | No validated result; fail closed as a backend invariant fault |

The canonical surface maps internal disagreement/bounds faults to
`OneShotError::Backend(BackendFault::ImpossibleState)`. Historical fast-path
failures fall back to the original decoder/validator after any quarantine;
its preflight bounds failures map to opaque `DecodeError::InvalidInput`.
Neither mapping changes ordinary malformed-input diagnostics. A rejection by
the portable validator followed by reference acceptance also fails closed with
`ImpossibleState`. Successful portable validation does not rerun the original
validator; callers wanting that pass select `ScalarReference`. Candidate vector
classifiers cannot authorize writes without complete scalar tail validation.
Commit 9 binds backend identity and applies health admission and quarantine.
Checked builds independently validate the whole input when preparing a proof and
compare bounded output chunks with the original scalar decoder. Kernel
rejection or checked output mismatch quarantines the backend and rewrites the
body with the validated table path; no error is returned after partial writes.
The test-only fault hooks use thread-local quarantine observations rather than
poisoning shared process health during parallel tests.

The policy gate also runs exhaustive short-input and tail tests, an independent
bounded layout model, `usize::MAX` arithmetic checks, fault injection through
the preflight/write path, and compiler rejection tests for source mutation,
proof reuse, and configuration/input substitution on active Rust and the MSRV.

## SSSE3 Validation Candidate History

Commit 6 adds test-only Standard/URL-safe validation of complete 16-byte blocks
without output stores. The final 1-16 input bytes stay with the original scalar
validator, including canonical padding/tail checks. The private preflight binds
the successful validation before the existing direct decode kernel writes.
On rejection, whole-input reference validation recovers the original error.
Custom/relaxed settings and unavailable CPUs use the reference path.

The original evaluation route is compiled only for x86 unit tests with `std,simd`; it cannot
be selected by public `Auto`, static tokens, CT APIs or production callers.
Commit 9 separately promotes its classifier into the production fault-handling
route above. Run `sh scripts/check-2.1-ssse3-validation.sh` for
the candidate's native tests and generated-code checks. This is not performance
admission. A test assertion catches decode-kernel disagreement in the original
evaluation route, which remains excluded from production.

Kani equivalence for the portable validator would provide additional bounded
assurance; it is not claimed by the current exhaustive classification/tail and
reference/oracle tests.

## AVX2 Validation Candidate History

Commit 7 adds the corresponding test-only AVX2 route. A bulk validator checks
32-byte blocks with a full YMM movemask; every lane in both 128-bit halves must
belong to the selected alphabet. The final 1-32 bytes use original scalar
validation. Empty input has no remainder. The complete immutable input and exact
settings are bound to preflight before capacity checks and output stores.
Rejection recovers whole-input reference diagnostics, including original indexes.

The writer uses the existing AVX2 direct-block loop and scalar residual quanta.
CPU/OS probing, complete-block geometry, exact output capacity and cleanup are
retained. Custom/relaxed settings or unavailable AVX2 use the reference path.
Like SSSE3, the candidate is test-only and uses an assertion for kernel
disagreement. It is not a new public API, checked-backend replacement, or an
admitted production fault-recovery path. Commit 9 separately promotes its bulk
classifier and adds production recovery; CT/secret engines and the explicit
reference validator remain separate.

Run `sh scripts/check-2.1-avx2-validation.sh` for exhaustive classification,
malformed-input parity, guard pages on Linux x86_64, active/MSRV feature builds,
assembly checks and production-IR exclusion of the asserting candidate (the
AVX2 classifier itself now ships). Set
`BASE64_NG_REQUIRE_AVX2_VALIDATION=1` when native execution must be mandatory.
The opt-in same-process benchmark and its limitations are described in
[Commit 7 measurements](PERFORMANCE_2.1_AVX2.md). Small-message overhead is not an
admission claim; dispatch thresholds remain a later checkpoint.

## AVX-512 Validation Candidate

Commit 8 adds a test-only 64-byte classifier and complete canonical candidate.
It retains the full AVX-512 F/BW/VL/VBMI CPU/OS-state probe, requires all 64
validity bits, and uses the existing exact 48-byte masked decode stores. The
original validator handles the final 1-64 bytes (zero for empty input), including
padding and canonical tail bits. Full-input reference diagnostics are recovered
on rejection. Shared preflight validates everything before any output write.

Only the four strict Standard/URL-safe families qualify; other settings and
unavailable hardware fall back to reference validation. Public automatic and
exact/static paths, explicit `ScalarReference`, and CT/secret APIs are unchanged.
The kernel-disagreement assertion is test-only, as in Commits 6 and 7; production
fault handling is required before integration, and automatic AVX-512 admission
remains a separate decision. Wider vectors alone do not establish a speedup.

Run `sh scripts/check-2.1-avx512-validation.sh` for active/MSRV feature matrices,
lane/byte and error-parity checks, Linux x86_64 guard pages, generated assembly
and fail-closed production-IR exclusion. Set
`BASE64_NG_REQUIRE_AVX512_VALIDATION=1` to require native execution. The opt-in
same-process AVX2 comparison is documented in
[Commit 8 measurements](PERFORMANCE_2.1_AVX512.md).

## NEON Validation

Commit 10 adds a little-endian AArch64 route. It classifies exact
16-byte blocks without writes, reduces every lane, uses the shared strict
scalar tail validator, and binds immutable input through preflight before
decoding. Rejection recovers exact reference diagnostics; custom/relaxed
settings stay on the reference path. Safe wrappers restrict alphabet selection
and block geometry and clear vector registers after success or rejection.

The integrated route uses a measured 4096-encoded-byte floor, shared health admission,
direct classifier KATs for both alphabets, and preflight-to-write health recheck.
False rejection detected by reference validation quarantines before mutation;
kernel rejection or checked-output disagreement quarantines and overwrites the
complete output through the scalar writer. `checked-backend` also compares
validation against the reference. Empty/small inputs retain their existing
paths. Explicit ScalarReference validation, static-token contracts, other
architectures, and secret/CT paths are unchanged.

The original asserting candidate remains test-only. Native AWS integrated
feature/MSRV, guard-page, fault-recovery and public-policy checks are retained
separately from operator-reported Apple Silicon results. The maintainer's
integrated Mac gate and all-features tests passed at `450239a`; paired integrated
Mac timings and external pentest/CI acceptance subsequently completed. See
[Commit 10 measurements and native commands](PERFORMANCE_2.1_NEON.md).

The NEON gate inspects production assembly with plain, checked and all-features
configurations, not only a unit-test build. Its fail-closed lane-mask and scalar
return model rejects an any-matching-lane reduction, wrong alphabet masks, and
disconnected or inverted Boolean returns. The external `decode_validation`
integration target also exercises every invalid byte in every NEON lane across
three vector blocks for all four strict presets, with no test-only library cfg.
It requires a healthy NEON backend and checks both validation and transactional
decode rejection. These checks supplement, rather than replace, native tests.

## Exact-Profile RVV Validation

Commit 12 connects the same preflight and recovery path to Linux SpacemiT X60
RVV. Public routing still requires the exact vendor/architecture/implementation
probe, enabled per-thread vector state and a healthy strict-decode KAT. Other
RISC-V profiles, non-Linux targets and safe no_std builds stay on their existing
fallbacks; a QEMU candidate cfg cannot admit this public route.

The classifier scans length-agnostic byte vectors, reducing all active lanes
before accepting. The safe wrapper requires complete 16-byte spans; the final
quantum and remaining tail retain scalar padding/canonical-bit validation.
The old 1024-byte decode crossover is applied to `len.saturating_sub(1) / 4 * 4`,
reserving the final complete or partial quantum. For valid inputs, the new
complete-call route starts at 1026 encoded bytes for unpadded profiles (a
two-byte tail), or 1028 for padded profiles. Unpadded 1027-byte inputs also
qualify; 1024-byte inputs remain below the crossover. The writer defensively
vector-classifies its span before using the old RVV packing leaf. Explicit
ScalarReference still independently validates input;
checked builds independently compare validation and decoded output.

Native X60 tests, dual-VLEN QEMU checks, production assembly contracts and
paired measurements are documented in [the RVV checkpoint](PERFORMANCE_2.1_RVV.md).
