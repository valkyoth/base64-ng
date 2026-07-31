use super::{
    CodecBuilder, DecodePadding, EncodePadding, InPlaceError, TrailingBits, ValidatedAlphabet,
    secret_in_place::{
        decode_with_injected_fault_for_test, reset_work_counters_for_test, work_counters_for_test,
    },
    specifications::{
        STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
        STRICT_URL_SAFE_UNPADDED,
    },
};

const SENTINEL: u8 = 0x5a;

#[test]
fn staged_secret_decode_enforces_preflight_fixed_work_and_cleanup() {
    assert_eq!(STRICT_STANDARD_PADDED.secret_decode_staging_len(0), Ok(0));
    assert_eq!(STRICT_STANDARD_PADDED.secret_decode_staging_len(1), Ok(3));
    assert_eq!(STRICT_STANDARD_PADDED.secret_decode_staging_len(4), Ok(3));
    assert_eq!(STRICT_STANDARD_PADDED.secret_decode_staging_len(5), Ok(6));

    exercise_valid_profiles();
    exercise_secret_semantics_match_strict_validation();
    exercise_preflight_is_non_destructive();
    exercise_invalid_input_and_fixed_work();
    exercise_internal_fault_cleanup();
    exercise_adjacent_same_page_storage();
}

fn exercise_secret_semantics_match_strict_validation() {
    exercise_mutations(&STRICT_STANDARD_PADDED, b"c2VjcmV0");
    exercise_mutations(&STRICT_STANDARD_PADDED, b"Zg==");
    exercise_mutations(&STRICT_STANDARD_UNPADDED, b"c2VjcmV0");
    exercise_mutations(&STRICT_STANDARD_UNPADDED, b"Zg");
}

fn exercise_mutations<S: super::Codec>(codec: &super::Base64<S>, canonical: &[u8]) {
    for index in 0..canonical.len() {
        for byte in u8::MIN..=u8::MAX {
            let mut input = [SENTINEL; 16];
            input[..canonical.len()].copy_from_slice(canonical);
            input[index] = byte;
            let original = input;
            let mut ordinary = [0u8; 16];
            let expected = codec.decode_into(&input[..canonical.len()], &mut ordinary);
            let mut staging = [SENTINEL; 16];
            let actual = codec.decode_in_place_staged(&mut input, canonical.len(), &mut staging);

            if let Ok(written) = expected {
                assert_eq!(actual, Ok(written), "index={index} byte={byte:#04x}");
                assert_eq!(&input[..written], &ordinary[..written]);
            } else {
                assert_eq!(
                    actual,
                    Err(InPlaceError::InvalidSecretInput),
                    "index={index} byte={byte:#04x}"
                );
                assert_eq!(input, original);
            }
            assert!(staging.iter().all(|candidate| *candidate == 0));
        }
    }
}

#[test]
fn staged_secret_decode_miri_overlap_contract() {
    let mut storage = [SENTINEL; 24];
    storage[..8].copy_from_slice(b"c2VjcmV0");
    let (buffer, staging) = storage.split_at_mut(12);
    let written = STRICT_STANDARD_UNPADDED
        .decode_in_place_staged(buffer, 8, staging)
        .unwrap();
    assert_eq!(&buffer[..written], b"secret");
    assert!(staging.iter().all(|byte| *byte == 0));
}

fn exercise_valid_profiles() {
    for len in 0..=96 {
        let mut plain = [0u8; 96];
        fill_pattern(&mut plain[..len]);
        exercise_valid(&STRICT_STANDARD_PADDED, &plain[..len]);
        exercise_valid(&STRICT_STANDARD_UNPADDED, &plain[..len]);
        exercise_valid(&STRICT_URL_SAFE_PADDED, &plain[..len]);
        exercise_valid(&STRICT_URL_SAFE_UNPADDED, &plain[..len]);
    }

    let alphabet = ValidatedAlphabet::new(
        *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
    )
    .unwrap();
    let codec = CodecBuilder::new(alphabet)
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Forbid)
        .trailing_bits(TrailingBits::RequireCanonical)
        .build()
        .unwrap();
    exercise_valid(&codec, b"custom-secret");
}

fn exercise_valid<S: super::Codec>(codec: &super::Base64<S>, plain: &[u8]) {
    let mut encoded = [SENTINEL; 160];
    let encoded_len = codec.encode_into(plain, &mut encoded).unwrap();
    let original = encoded;
    let mut staging = [SENTINEL; 128];
    let written = codec
        .decode_in_place_staged(&mut encoded, encoded_len, &mut staging)
        .unwrap();
    assert_eq!(&encoded[..written], plain);
    assert_eq!(&encoded[encoded_len..], &original[encoded_len..]);
    assert!(staging.iter().all(|byte| *byte == 0));
}

fn exercise_preflight_is_non_destructive() {
    let alphabet = ValidatedAlphabet::new(
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    )
    .unwrap();
    let unsupported = CodecBuilder::new(alphabet)
        .decode_padding(DecodePadding::Indifferent)
        .build()
        .unwrap();

    for case in 0..3 {
        let mut buffer = [SENTINEL; 12];
        buffer[..8].copy_from_slice(b"c2VjcmV0");
        let original_buffer = buffer;
        let mut staging = [0x3c; 12];
        let original_staging = staging;
        reset_work_counters_for_test();
        let result = match case {
            0 => STRICT_STANDARD_UNPADDED.decode_in_place_staged(&mut buffer, 13, &mut staging),
            1 => STRICT_STANDARD_UNPADDED.decode_in_place_staged(&mut buffer, 8, &mut staging[..5]),
            _ => unsupported.decode_in_place_staged(&mut buffer, 8, &mut staging),
        };
        assert!(matches!(
            result,
            Err(InPlaceError::InputLengthExceedsBuffer { .. }
                | InPlaceError::StagingTooSmall { .. }
                | InPlaceError::SecretPolicyUnsupported)
        ));
        assert_eq!(buffer, original_buffer);
        assert_eq!(staging, original_staging);
        assert_eq!(work_counters_for_test(), (0, 0));
    }
}

fn exercise_invalid_input_and_fixed_work() {
    let mut valid = [SENTINEL; 16];
    valid[..8].copy_from_slice(b"c2VjcmV0");
    let mut valid_staging = [SENTINEL; 12];
    reset_work_counters_for_test();
    assert_eq!(
        STRICT_STANDARD_UNPADDED.decode_in_place_staged(&mut valid, 8, &mut valid_staging),
        Ok(6)
    );
    let valid_work = work_counters_for_test();

    let mut invalid = [SENTINEL; 16];
    invalid[..8].copy_from_slice(b"c2Vj!mV0");
    let original = invalid;
    let mut invalid_staging = [SENTINEL; 12];
    reset_work_counters_for_test();
    assert_eq!(
        STRICT_STANDARD_UNPADDED.decode_in_place_staged(&mut invalid, 8, &mut invalid_staging,),
        Err(InPlaceError::InvalidSecretInput)
    );
    assert_eq!(invalid, original);
    assert!(invalid_staging.iter().all(|byte| *byte == 0));
    assert_eq!(work_counters_for_test(), valid_work);
    assert_eq!(valid_work, (1, 8));
}

fn exercise_internal_fault_cleanup() {
    let mut buffer = [SENTINEL; 16];
    buffer[..8].copy_from_slice(b"c2VjcmV0");
    let mut staging = [SENTINEL; 12];
    assert_eq!(
        decode_with_injected_fault_for_test(
            &STRICT_STANDARD_UNPADDED,
            &mut buffer,
            8,
            &mut staging,
        ),
        Err(InPlaceError::Backend(super::BackendFault::ImpossibleState))
    );
    assert!(buffer.iter().all(|byte| *byte == 0));
    assert!(staging.iter().all(|byte| *byte == 0));
}

fn exercise_adjacent_same_page_storage() {
    let mut storage = [SENTINEL; 64];
    storage[..8].copy_from_slice(b"c2VjcmV0");
    let (buffer, staging) = storage.split_at_mut(32);
    assert_eq!(
        STRICT_STANDARD_UNPADDED.decode_in_place_staged(buffer, 8, staging),
        Ok(6)
    );
    assert_eq!(&buffer[..6], b"secret");
    assert!(staging.iter().all(|byte| *byte == 0));
}

fn fill_pattern(bytes: &mut [u8]) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .unwrap_or(0)
            .wrapping_mul(41)
            .wrapping_add(7);
    }
}
