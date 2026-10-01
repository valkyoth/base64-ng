# Ordinary Decode Validation

2.1 adds the non-exhaustive `DecodeValidation::{Auto, ScalarReference}` policy.
It is a per-call choice, not a Cargo feature or a change to the codec grammar.
`Auto` uses health-gated SSSE3/SSE4.1 or AVX2 validation and writing for strict
Standard/URL-safe padded/unpadded presets and exactly equivalent runtime
settings. Canonical fallback uses portable table validation; `ScalarReference`
retains the original validator. Eligible historical x86 calls share the fast
core without acquiring canonical error semantics. Automatic AVX-512, NEON and
wasm vector validation are not introduced by Commit 9.

The new vector route starts at 512 encoded bytes; smaller canonical calls keep
portable validation/writing and smaller historical calls keep their existing
decoder. This conservative complete-call cutoff avoids measured small-message
setup regressions; it does not change existing static/exact ISA contracts.

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
| Canonical validation and exact length | `validate_with_validation`, `decoded_len_with_validation` |
| Canonical allocating helpers | `decode_to_vec_with_validation`, `decode_to_vec_with_limit_and_validation` |
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
- Historical `Auto` shares the fast core for eligible x86 calls with sufficient
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
  tokens nor overrides execution/deployment restrictions. No token overload or
  change to incremental `decoder()` semantics is introduced in this checkpoint.
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
Canonical caller-buffer and allocating decode share this boundary. Allocating
decode now retains the result across reservation instead of validating again.
Historical validation-only helpers share its length checks. Commit 9 also
routes eligible historical ordinary decoding through it. Incremental states,
static tokens and CT/secret paths are not rerouted.

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
Checked builds independently validate the whole input before writing and
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

## NEON Validation Candidate

Commit 10 adds a test-only little-endian AArch64 route. It classifies exact
16-byte blocks without writes, reduces every lane, uses the shared strict
scalar tail validator, and binds immutable input through preflight before
decoding. Rejection recovers exact reference diagnostics; custom/relaxed
settings stay on the reference path. Safe wrappers restrict alphabet selection
and block geometry and clear vector registers after success or rejection.

This candidate does not change production NEON, static no_std, checked-backend,
ScalarReference or secret/CT routing. Its test-only disagreement assertion
requires production health/quarantine integration before promotion. Native AWS
and QEMU correctness are recorded separately; Apple Silicon verification is
pending. See [Commit 10 measurements and native commands](PERFORMANCE_2.1_NEON.md).
