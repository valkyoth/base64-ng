#!/usr/bin/env sh
set -eu

review="docs/SIMD_NON_STANDARD_SURFACE_REVIEW.md"
manifest="docs/SIMD_ADMISSION.md"
plan="docs/PLAN.md"

test -s "$review"

for required_text in \
    "Wrapped encode staging and wrapped decode's compacted strict decode stage are" \
    "custom alphabet encode and decode" \
    "bcrypt-style and \`crypt(3)\`-style alphabet encode and decode" \
    "strict in-place decode through stack staging" \
    "in-place encode through stack staging" \
    "legacy-whitespace decode's compacted strict decode stage" \
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
    "| MIME/PEM wrapped encode | admitted for unwrapped staging only |" \
    "| MIME/PEM wrapped decode | admitted for compacted strict decode only |" \
    "| Legacy-whitespace decode | admitted for compacted strict decode only |" \
    "| In-place encode | admitted for stack-staged Standard/URL-safe encode only |" \
    "| In-place decode | admitted for stack-staged Standard/URL-safe strict decode only |" \
    "| Constant-time-oriented secret decode | scalar only |" \
    "Engine::decode_slice_legacy" \
    "legacy decode compacts into strict chunks" \
    "Engine::decode_slice_wrapped" \
    "decodes those strict chunks through \`Engine::decode_slice\`" \
    "Engine::encode_slice_wrapped" \
    "write_wrapped_byte" \
    "write_wrapped_bytes" \
    "in-place encode uses stack staging" \
    "in-place decode uses stack staging" \
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

if ! grep -F -q "decode_legacy_via_strict_backend(input, output)" src/engine/decode.rs; then
    echo "simd non-standard surfaces: legacy decode must route through compacted strict backend staging" >&2
    exit 1
fi

if ! grep -F -q "is_legacy_whitespace(*byte)" src/engine/decode.rs; then
    echo "simd non-standard surfaces: legacy decode staging must keep scalar whitespace compaction" >&2
    exit 1
fi

if ! grep -F -q "decode_wrapped_via_strict_backend(input, output, wrap)" src/engine/decode.rs; then
    echo "simd non-standard surfaces: wrapped decode must route through compacted strict backend staging" >&2
    exit 1
fi

if ! grep -F -q "decode_backend::decode_slice::<A, PAD>" src/engine/decode.rs; then
    echo "simd non-standard surfaces: wrapped decode staging must enter strict decode backend" >&2
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

if ! grep -F -q "encode_in_place_staged::<A, PAD>" src/encode_backend.rs; then
    echo "simd non-standard surfaces: in-place encode must route through stack staging before admitted backend use" >&2
    exit 1
fi

if ! grep -F -q "scalar_encode_in_place::encode_in_place::<A, PAD>" src/encode_backend.rs; then
    echo "simd non-standard surfaces: unsupported in-place encode must keep scalar fallback" >&2
    exit 1
fi

if ! grep -F -q "input_scratch[..chunk_len].copy_from_slice" src/encode_backend.rs; then
    echo "simd non-standard surfaces: in-place encode must copy unread input into scratch before writing output" >&2
    exit 1
fi

if ! grep -F -q "decode_backend::decode_slice::<A, PAD>" src/engine/decode_in_place.rs; then
    echo "simd non-standard surfaces: in-place decode must route staged chunks through strict decode backend" >&2
    exit 1
fi

if ! grep -F -q "scratch[..chunk_len].copy_from_slice" src/engine/decode_in_place.rs; then
    echo "simd non-standard surfaces: in-place decode must copy encoded input into scratch before backend decode" >&2
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

if ! grep -F -q "fn decode_in_place_handles_large_chunked_staging()" tests/rfc4648.rs; then
    echo "simd non-standard surfaces: missing chunked in-place decode regression evidence" >&2
    exit 1
fi

echo "simd non-standard surfaces: ok"
