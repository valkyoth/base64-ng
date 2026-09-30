# Ordinary Decode Validation

2.1 adds the non-exhaustive `DecodeValidation::{Auto, ScalarReference}` policy.
It is a per-call choice, not a Cargo feature or a change to the codec grammar.
The initial implementation uses the existing scalar validation for both choices;
it does not yet enable accelerated validation or claim a throughput improvement.

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

- `ScalarReference` means scalar grammar checks, not scalar output instructions.
  Valid input is fully checked. Malformed input can fail early; this is not CT.
  The existing scalar decoder combines validation with output generation;
  SIMD decoders retain scalar prevalidation. This preserves historical behavior
  without adding a redundant extra pass to every scalar decode.
- `Auto` currently follows those same paths. Future acceleration requires its
  own admission; the explicit reference choice remains available.
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
observations verify that reference work runs in the actual scalar routines.
The existing 2.0.4 downstream fixture remains unchanged.
