//! Bounded proofs for the final 2.0 ordinary core.

#[cfg(base64_ng_kani_advanced)]
use crate::v2::{Base64, Codec, ValidatedAlphabet, tail_is_canonical_for_proof};
use crate::v2::{
    CodecBuilder, DecodePadding, EncodePadding, InPlaceError, STRICT_STANDARD_PADDED,
    STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, Status,
    TrailingBits, pack_full_quantum_for_proof, require_in_place_disjoint_ranges_for_proof,
};

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum OracleProfile {
    StandardPadded,
    StandardUnpadded,
    UrlSafePadded,
    UrlSafeUnpadded,
}

impl OracleProfile {
    const fn alphabet(self) -> &'static [u8; 64] {
        match self {
            Self::StandardPadded | Self::StandardUnpadded => STANDARD,
            Self::UrlSafePadded | Self::UrlSafeUnpadded => URL_SAFE,
        }
    }

    const fn padded(self) -> bool {
        matches!(self, Self::StandardPadded | Self::UrlSafePadded)
    }
}

fn oracle_encode(profile: OracleProfile, input: &[u8], output: &mut [u8; 4]) -> usize {
    let alphabet = profile.alphabet();
    match input {
        [] => 0,
        [first] => {
            output[0] = alphabet[usize::from(first >> 2)];
            output[1] = alphabet[usize::from((first & 3) << 4)];
            if profile.padded() {
                output[2] = b'=';
                output[3] = b'=';
                4
            } else {
                2
            }
        }
        [first, second] => {
            output[0] = alphabet[usize::from(first >> 2)];
            output[1] = alphabet[usize::from(((first & 3) << 4) | (second >> 4))];
            output[2] = alphabet[usize::from((second & 15) << 2)];
            if profile.padded() {
                output[3] = b'=';
                4
            } else {
                3
            }
        }
        [first, second, third] => {
            output[0] = alphabet[usize::from(first >> 2)];
            output[1] = alphabet[usize::from(((first & 3) << 4) | (second >> 4))];
            output[2] = alphabet[usize::from(((second & 15) << 2) | (third >> 6))];
            output[3] = alphabet[usize::from(third & 63)];
            4
        }
        _ => unreachable!("the bounded oracle accepts at most one quantum"),
    }
}

#[cfg(base64_ng_kani_advanced)]
fn assert_preset_encode_matches_oracle<S: Codec>(codec: &Base64<S>, profile: OracleProfile) {
    let input = kani::any::<[u8; 3]>();
    let input_len = usize::from(kani::any::<u8>() % 4);
    let mut expected = [0u8; 4];
    let expected_len = oracle_encode(profile, &input[..input_len], &mut expected);
    let mut encoded = [0u8; 4];
    let written = codec
        .encode_into(&input[..input_len], &mut encoded)
        .expect("one bounded quantum fits");

    assert!(written == expected_len);
    assert!(encoded[..written] == expected[..expected_len]);
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn strict_standard_padded_encode_refines_independent_oracle() {
    assert_preset_encode_matches_oracle(&STRICT_STANDARD_PADDED, OracleProfile::StandardPadded);
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn strict_standard_unpadded_encode_refines_independent_oracle() {
    assert_preset_encode_matches_oracle(&STRICT_STANDARD_UNPADDED, OracleProfile::StandardUnpadded);
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn strict_url_safe_padded_encode_refines_independent_oracle() {
    assert_preset_encode_matches_oracle(&STRICT_URL_SAFE_PADDED, OracleProfile::UrlSafePadded);
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn strict_url_safe_unpadded_encode_refines_independent_oracle() {
    assert_preset_encode_matches_oracle(&STRICT_URL_SAFE_UNPADDED, OracleProfile::UrlSafeUnpadded);
}

#[kani::proof]
#[kani::unwind(66)]
fn standard_alphabet_lookup_refines_symbol_position() {
    let index = kani::any::<u8>() & 63;
    let settings = STRICT_STANDARD_PADDED.settings();
    let alphabet = settings.alphabet();
    let encoded = alphabet.as_array()[usize::from(index)];
    assert!(alphabet.decode_byte(encoded) == Some(index));
}

#[kani::proof]
#[kani::unwind(66)]
fn url_safe_alphabet_lookup_refines_symbol_position() {
    let index = kani::any::<u8>() & 63;
    let settings = STRICT_URL_SAFE_PADDED.settings();
    let alphabet = settings.alphabet();
    let encoded = alphabet.as_array()[usize::from(index)];
    assert!(alphabet.decode_byte(encoded) == Some(index));
}

#[kani::proof]
fn production_decode_packing_refines_independent_formula() {
    let values = kani::any::<[u8; 4]>();
    kani::assume(values[0] < 64);
    kani::assume(values[1] < 64);
    kani::assume(values[2] < 64);
    kani::assume(values[3] < 64);
    let expected = [
        (values[0] << 2) | (values[1] >> 4),
        (values[1] << 4) | (values[2] >> 2),
        (values[2] << 6) | values[3],
    ];
    assert!(pack_full_quantum_for_proof(values) == expected);
}

#[kani::proof]
fn strict_aliases_select_the_exported_policies() {
    assert!(crate::STRICT_STANDARD_PADDED.settings() == STRICT_STANDARD_PADDED.settings());
    assert!(crate::STRICT_STANDARD_UNPADDED.settings() == STRICT_STANDARD_UNPADDED.settings());
    assert!(crate::STRICT_URL_SAFE_PADDED.settings() == STRICT_URL_SAFE_PADDED.settings());
    assert!(crate::STRICT_URL_SAFE_UNPADDED.settings() == STRICT_URL_SAFE_UNPADDED.settings());
}

#[kani::proof]
fn final_core_encoded_lengths_match_independent_formula() {
    let len = usize::from(kani::any::<u8>());
    let complete = len / 3 * 4;
    let remainder = len % 3;
    let padded = complete + usize::from(remainder != 0) * 4;
    let unpadded = complete + if remainder == 0 { 0 } else { remainder + 1 };

    assert!(STRICT_STANDARD_PADDED.encoded_len(len) == Ok(padded));
    assert!(STRICT_URL_SAFE_PADDED.encoded_len(len) == Ok(padded));
    assert!(STRICT_STANDARD_UNPADDED.encoded_len(len) == Ok(unpadded));
    assert!(STRICT_URL_SAFE_UNPADDED.encoded_len(len) == Ok(unpadded));
}

#[kani::proof]
#[kani::unwind(70)]
fn runtime_policy_product_is_validated_before_use() {
    let alphabet = *STRICT_STANDARD_PADDED.settings().alphabet();
    let encode = if kani::any::<bool>() {
        EncodePadding::Padded
    } else {
        EncodePadding::Unpadded
    };
    let decode = match kani::any::<u8>() % 3 {
        0 => DecodePadding::RequireCanonical,
        1 => DecodePadding::Forbid,
        _ => DecodePadding::Indifferent,
    };
    let trailing = if kani::any::<bool>() {
        TrailingBits::RequireCanonical
    } else {
        TrailingBits::AllowNonCanonical
    };
    let result = CodecBuilder::new(alphabet)
        .encode_padding(encode)
        .decode_padding(decode)
        .trailing_bits(trailing)
        .build();
    let conflicting = matches!(
        (encode, decode),
        (EncodePadding::Padded, DecodePadding::Forbid)
            | (EncodePadding::Unpadded, DecodePadding::RequireCanonical)
    );

    assert!(result.is_err() == conflicting);
    if let Ok(codec) = result {
        let settings = codec.settings();
        assert!(settings.encode_padding() == encode);
        assert!(settings.decode_padding() == decode);
        assert!(settings.trailing_bits() == trailing);
        assert!(
            settings.permits_secret_processing()
                == (!matches!(decode, DecodePadding::Indifferent)
                    && matches!(trailing, TrailingBits::RequireCanonical))
        );
    }
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
fn strict_tail_predicates_match_rfc_canonical_masks() {
    let value = kani::any::<u8>() & 63;
    assert!(tail_is_canonical_for_proof(value, 1) == (value & 0x0f == 0));
    assert!(tail_is_canonical_for_proof(value, 2) == (value & 0x03 == 0));
    assert!(!tail_is_canonical_for_proof(value, 0));
    assert!(!tail_is_canonical_for_proof(value, 3));
}

#[kani::proof]
#[kani::unwind(20)]
fn incremental_encode_refines_independent_oracle() {
    let input = kani::any::<[u8; 3]>();
    let input_len = usize::from(kani::any::<u8>() % 4);
    let mut expected = [0u8; 4];
    let expected_len = oracle_encode(
        OracleProfile::StandardPadded,
        &input[..input_len],
        &mut expected,
    );
    let mut output = [0u8; 4];
    let mut state = STRICT_STANDARD_PADDED.encoder();
    let update = state
        .update(&input[..input_len], &mut output)
        .expect("ordinary byte input is infallible");
    let produced = update.progress().output_produced();
    let finish = state
        .finish(&mut output[produced..])
        .expect("bounded output is exact");
    let total = produced + finish.progress().output_produced();

    assert!(matches!(finish.status(), Status::Complete));
    assert!(total == expected_len);
    assert!(output[..total] == expected[..expected_len]);
    assert!(state.proof_invariants());
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn incremental_decode_matches_rfc_known_answer_all_tail_lengths() {
    let input = [0x00, 0x29, 0x52];
    let input_len = usize::from(kani::any::<u8>() % 4);
    let mut encoded = [0u8; 4];
    let encoded_len = oracle_encode(
        OracleProfile::StandardPadded,
        &input[..input_len],
        &mut encoded,
    );
    let mut output = [0u8; 3];
    let mut state = STRICT_STANDARD_PADDED.decoder();
    let update = state
        .update(&encoded[..encoded_len], &mut output)
        .expect("oracle input is canonical");
    let produced = update.progress().output_produced();
    let finish = state
        .finish(&mut output[produced..])
        .expect("canonical input finalizes");
    let total = produced + finish.progress().output_produced();

    assert!(matches!(finish.status(), Status::Complete));
    assert!(total == input_len);
    assert!(output[..total] == input[..input_len]);
    assert!(state.proof_invariants());
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn in_place_encode_decode_matches_rfc_known_answer_all_tail_lengths() {
    let input = [0x00, 0x29, 0x52];
    let input_len = usize::from(kani::any::<u8>() % 4);
    let mut expected = [0u8; 4];
    let expected_len = oracle_encode(
        OracleProfile::StandardPadded,
        &input[..input_len],
        &mut expected,
    );
    let mut buffer = [0u8; 4];
    buffer[..input_len].copy_from_slice(&input[..input_len]);
    let written = STRICT_STANDARD_PADDED
        .encode_in_place(&mut buffer, input_len)
        .expect("one encoded quantum fits");
    assert!(written == expected_len);
    assert!(buffer[..written] == expected[..expected_len]);

    let decoded = STRICT_STANDARD_PADDED
        .decode_in_place(&mut buffer, written)
        .expect("oracle encoding is canonical");
    assert!(decoded == input_len);
    assert!(buffer[..decoded] == input[..input_len]);
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn in_place_validation_error_rolls_back_complete_buffer() {
    let mut buffer = [b'!', b'A', b'A', b'A'];
    let before = buffer;
    let result = STRICT_STANDARD_PADDED.decode_in_place(&mut buffer, 4);

    assert!(result.is_err());
    assert!(buffer == before);
}

#[kani::proof]
fn in_place_range_preflight_matches_mathematical_overlap() {
    let left_start = kani::any::<usize>();
    let left_len = usize::from(kani::any::<u8>());
    let right_start = kani::any::<usize>();
    let right_len = usize::from(kani::any::<u8>());
    let result =
        require_in_place_disjoint_ranges_for_proof(left_start, left_len, right_start, right_len);
    let Some(left_end) = left_start.checked_add(left_len) else {
        assert!(matches!(result, Err(InPlaceError::AddressRangeOverflow)));
        return;
    };
    let Some(right_end) = right_start.checked_add(right_len) else {
        assert!(matches!(result, Err(InPlaceError::AddressRangeOverflow)));
        return;
    };
    let overlaps =
        left_len != 0 && right_len != 0 && left_start < right_end && right_start < left_end;

    assert!(matches!(result, Err(InPlaceError::OverlappingBuffers)) == overlaps);
    assert!(result.is_ok() == !overlaps);
}

#[kani::proof]
#[kani::unwind(20)]
fn incremental_finalization_retries_make_bounded_progress() {
    let input = kani::any::<[u8; 2]>();
    let mut state = STRICT_STANDARD_PADDED.encoder();
    let update = state
        .update(&input, &mut [])
        .expect("ordinary byte input is infallible");
    assert!(update.progress().input_consumed() == input.len());

    let mut output = [0u8; 4];
    let mut produced = 0usize;
    while produced < output.len() {
        let finish = state
            .finish(&mut output[produced..produced + 1])
            .expect("one byte always drains pending final output");
        assert!(finish.progress().input_consumed() == 0);
        assert!(finish.progress().output_produced() == 1);
        produced += 1;
    }
    let complete = state.finish(&mut []).expect("completed state is stable");
    assert!(matches!(complete.status(), Status::Complete));
    assert!(complete.progress().output_produced() == 0);
    assert!(state.proof_invariants());
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
fn validated_builtin_alphabet_is_a_runtime_builder_fixed_point() {
    let table = *STRICT_STANDARD_PADDED.settings().alphabet().as_array();
    let validated = ValidatedAlphabet::new(table).expect("RFC 4648 alphabet is valid");
    assert!(validated.as_array() == &table);
}
