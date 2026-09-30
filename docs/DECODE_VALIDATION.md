# Ordinary Decode Validation

2.1 adds the non-exhaustive `DecodeValidation::{Auto, ScalarReference}` policy.
It is a per-call choice, not a Cargo feature or a change to the codec grammar.
Canonical `Auto` specializes portable scalar validation for strict Standard and
URL-safe padded/unpadded presets and exactly equivalent runtime settings.
`ScalarReference` retains the original validator. Historical decode paths are
unchanged; no new vector validation is enabled by this checkpoint.

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
  SIMD decoders retain scalar prevalidation. This preserves historical behavior
  without adding a redundant extra pass to every scalar decode.
- Canonical `Auto` checks complete grammar with input-indexed lookup tables,
  then writes through the private preflight result. On rejection, the original
  validator recovers exact diagnostics before capacity or allocation checks.
  Custom alphabets and relaxed settings retain the original validator and
  writer. Empty input is accepted without inspecting the alphabet.
- Canonical writing uses the same specialized tables under either validation
  policy when settings qualify. Reference validation still runs the original
  incremental state machine, not the optimized table validator.
- Historical `Auto` still follows the existing paths. Future vector validation
  requires its own admission; the explicit reference choice remains available.
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
Historical validation-only helpers share its length checks; historical decode,
checked-backend comparisons, incremental states, and CT/secret paths are not
rerouted.

Checked geometry reserves the last quantum (at most four input bytes and three
output bytes), leaving only complete unpadded quanta in the interior. Empty
input, impossible lengths, arithmetic bounds and the measured output length
are checked before the destination is sliced. The geometry check is not a
grammar validator: a complete surface-specific validator must succeed first.
Commit 5 adds a reviewed portable validator for the exact canonical settings;
custom alphabets, relaxed settings and explicit `ScalarReference` keep the
original validation. The fast validator also verifies padding placement and
canonical tail bits; it is not merely an interior-block classifier.

The future vector classifier/reference disagreement contract remains covered
with test-only injection (no vector classifier is enabled here):

| Classifier vs reference | Outcome before any write |
| --- | --- |
| Both reject | Retain the reference's exact surface-specific diagnostic |
| Both accept | Require checked span bounds and destination capacity |
| Either disagrees | No validated result; fail closed as a backend invariant fault |

The canonical surface maps internal disagreement/bounds faults to
`OneShotError::Backend(BackendFault::ImpossibleState)`; the historical
validation-only surface maps them to opaque `DecodeError::InvalidInput`.
Neither mapping changes ordinary malformed-input diagnostics. A rejection by
the portable validator followed by reference acceptance also fails closed with
`ImpossibleState`. Successful portable validation does not rerun the original
validator; callers wanting that pass select `ScalarReference`. Candidate vector
classifiers cannot authorize writes on their own. Future vector integration
must associate faults with backend identity and quarantine before admission;
existing checked-output quarantine and retry behavior is unchanged.

The policy gate also runs exhaustive short-input and tail tests, an independent
bounded layout model, `usize::MAX` arithmetic checks, fault injection through
the preflight/write path, and compiler rejection tests for source mutation,
proof reuse, and configuration/input substitution on active Rust and the MSRV.
