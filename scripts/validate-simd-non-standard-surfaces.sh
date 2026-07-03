#!/usr/bin/env sh
set -eu

review="docs/SIMD_NON_STANDARD_SURFACE_REVIEW.md"
manifest="docs/SIMD_ADMISSION.md"
plan="docs/PLAN.md"

test -s "$review"

for required_text in \
    "No new non-standard SIMD acceleration is admitted yet." \
    "custom alphabet encode and decode" \
    "bcrypt-style and \`crypt(3)\`-style alphabet encode and decode" \
    "strict in-place decode" \
    "legacy-whitespace decode" \
    "strict wrapped decode" \
    "wrapped encode staging" \
    "non_standard_simd_candidate_surfaces_preserve_scalar_behavior" \
    "non_standard_simd_candidate_error_surfaces_preserve_scalar_behavior" \
    "non_standard_simd_candidate_clear_tail_surfaces_preserve_scalar_behavior" \
    "non_standard_profile_surfaces_preserve_engine_routing" \
    "naive wrapped-output oracle" \
    "Named profiles" \
    "| Custom alphabet encode | scalar fallback |" \
    "| Custom alphabet decode | scalar fallback |" \
    "| Bcrypt and \`crypt(3)\` profiles | scalar fallback |" \
    "| MIME/PEM wrapped encode | partially staged through admitted unwrapped encode |" \
    "| MIME/PEM wrapped decode | scalar fallback |" \
    "| Legacy-whitespace decode | scalar fallback |" \
    "| In-place encode | scalar fallback |" \
    "| In-place decode | scalar fallback |" \
    "| Constant-time-oriented secret decode | scalar only |" \
    "Engine::decode_slice_legacy" \
    "decode_legacy_to_slice" \
    "Engine::decode_slice_wrapped" \
    "decode_wrapped_to_slice" \
    "Engine::encode_slice_wrapped" \
    "write_wrapped_byte" \
    "write_wrapped_bytes" \
    "Do not broaden public SIMD claims"
do
    if ! grep -F -q "$required_text" "$review"; then
        echo "simd non-standard surfaces: missing required text: $required_text" >&2
        exit 1
    fi
done

for policy_doc in "$manifest" "$plan" docs/SIMD.md docs/UNSAFE.md
do
    if ! grep -F -q "SIMD_NON_STANDARD_SURFACE_REVIEW.md" "$policy_doc"; then
        echo "simd non-standard surfaces: policy doc must link to review ledger: $policy_doc" >&2
        exit 1
    fi
done

if ! grep -F -q "decode_legacy_to_slice::<A, PAD>(input, output)" src/engine/decode.rs; then
    echo "simd non-standard surfaces: legacy decode must route through decode_legacy_to_slice" >&2
    exit 1
fi

if ! grep -F -q "decode_wrapped_to_slice::<A, PAD>(input, output, wrap)" src/engine/decode.rs; then
    echo "simd non-standard surfaces: wrapped decode must route through decode_wrapped_to_slice" >&2
    exit 1
fi

if ! grep -F -q ".encode_slice(&input[input_offset..input_offset + take], &mut scratch)" src/engine/encode.rs; then
    echo "simd non-standard surfaces: wrapped encode scratch path must use admitted unwrapped encode staging" >&2
    exit 1
fi

if ! grep -F -q "self.encode_slice(input, &mut output[required..required + encoded_len])" src/engine/encode.rs; then
    echo "simd non-standard surfaces: wrapped encode in-buffer path must use admitted unwrapped encode staging" >&2
    exit 1
fi

if ! grep -F -q "scalar_encode_in_place::encode_in_place::<A, PAD>" src/encode_backend.rs; then
    echo "simd non-standard surfaces: in-place encode must remain scalar-routed" >&2
    exit 1
fi

for required_test_text in \
    "fn non_standard_simd_candidate_surfaces_preserve_scalar_behavior()" \
    "fn non_standard_simd_candidate_error_surfaces_preserve_scalar_behavior()" \
    "fn non_standard_simd_candidate_clear_tail_surfaces_preserve_scalar_behavior()" \
    "fn non_standard_profile_surfaces_preserve_engine_routing()" \
    "assert_wrapped_encode_matches_unwrapped_then_wrap" \
    "assert_wrapped_profile_matches_engine" \
    "assert_unwrapped_profile_matches_engine" \
    "assert_slice_clear_tail_matches_scalar" \
    "assert_in_place_encode_matches_scalar" \
    "let take = (unwrapped_len - read).min(wrap.line_len())"
do
    if ! grep -F -q "$required_test_text" src/non_standard_surface_tests.rs; then
        echo "simd non-standard surfaces: missing regression test evidence: $required_test_text" >&2
        exit 1
    fi
done

echo "simd non-standard surfaces: ok"
