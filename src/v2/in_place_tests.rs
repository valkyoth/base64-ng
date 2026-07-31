use super::{
    CodecBuilder, InPlaceError, ValidatedAlphabet,
    in_place::require_disjoint_ranges_for_test,
    specifications::{
        Base64, Codec, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
        STRICT_URL_SAFE_UNPADDED,
    },
};

const SENTINEL: u8 = 0xa5;

#[test]
fn ordinary_in_place_matches_transactional_one_shot_for_all_bounded_lengths() {
    exercise_profile(&STRICT_STANDARD_PADDED);
    exercise_profile(&STRICT_STANDARD_UNPADDED);
    exercise_profile(&STRICT_URL_SAFE_PADDED);
    exercise_profile(&STRICT_URL_SAFE_UNPADDED);
    let custom = CodecBuilder::new(
        ValidatedAlphabet::new(
            *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
        )
        .unwrap(),
    )
    .build()
    .unwrap();
    exercise_profile(&custom);
}

#[test]
fn ordinary_preflight_and_input_errors_do_not_mutate() {
    let mut too_short = [SENTINEL; 7];
    too_short[..5].copy_from_slice(b"hello");
    let original = too_short;
    assert_eq!(
        STRICT_STANDARD_PADDED.encode_in_place(&mut too_short, 5),
        Err(InPlaceError::OutputTooSmall {
            required: 8,
            available: 7,
        })
    );
    assert_eq!(too_short, original);

    let mut invalid_prefix = *b"helloXXX";
    let original = invalid_prefix;
    assert_eq!(
        STRICT_STANDARD_PADDED.encode_in_place(&mut invalid_prefix, 9),
        Err(InPlaceError::InputLengthExceedsBuffer {
            input_len: 9,
            buffer_len: 8,
        })
    );
    assert_eq!(invalid_prefix, original);

    for malformed in [
        b"!!!!".as_slice(),
        b"Zg=A".as_slice(),
        b"Zh==".as_slice(),
        b"Zg=".as_slice(),
        b"Zg==AAAA".as_slice(),
    ] {
        let mut buffer = [SENTINEL; 16];
        buffer[..malformed.len()].copy_from_slice(malformed);
        let original = buffer;
        assert!(
            STRICT_STANDARD_PADDED
                .decode_in_place(&mut buffer, malformed.len())
                .is_err()
        );
        assert_eq!(buffer, original, "malformed={malformed:?}");
    }
}

#[test]
fn checked_byte_ranges_define_every_overlap_boundary() {
    assert_eq!(
        require_disjoint_ranges_for_test(0x1000, 16, 0x1010, 16),
        Ok(())
    );
    assert_eq!(
        require_disjoint_ranges_for_test(0x1010, 16, 0x1000, 16),
        Ok(())
    );
    assert_eq!(
        require_disjoint_ranges_for_test(0x1000, 0, 0x1000, 16),
        Ok(())
    );
    assert_eq!(
        require_disjoint_ranges_for_test(0x1000, 16, 0x1008, 16),
        Err(InPlaceError::OverlappingBuffers)
    );
    assert_eq!(
        require_disjoint_ranges_for_test(0x1008, 16, 0x1000, 16),
        Err(InPlaceError::OverlappingBuffers)
    );
    assert_eq!(
        require_disjoint_ranges_for_test(0x1000, 16, 0x1000, 16),
        Err(InPlaceError::OverlappingBuffers)
    );
    assert_eq!(
        require_disjoint_ranges_for_test(usize::MAX - 3, 4, 0x1000, 1),
        Err(InPlaceError::AddressRangeOverflow)
    );
    assert_eq!(
        require_disjoint_ranges_for_test(0x1000, 1, usize::MAX - 1, 2),
        Err(InPlaceError::AddressRangeOverflow)
    );

    // Byte-disjoint slices may share one effective page. Commit 40 owns page
    // accounting; Commit 14 rejects only actual byte overlap.
    assert_eq!(
        require_disjoint_ranges_for_test(0x1003, 10, 0x1010, 10),
        Ok(())
    );
}

fn exercise_profile<S: Codec>(codec: &Base64<S>) {
    for len in 0..=193 {
        let mut input = [0u8; 193];
        fill_pattern(&mut input[..len]);

        let required = codec.encoded_len(len).unwrap();
        let mut expected_encoded = [SENTINEL; 260];
        let expected_written = codec
            .encode_into(&input[..len], &mut expected_encoded)
            .unwrap();
        assert_eq!(required, expected_written);

        let mut encoded = [SENTINEL; 260];
        encoded[..len].copy_from_slice(&input[..len]);
        let written = codec.encode_in_place(&mut encoded, len).unwrap();
        assert_eq!(written, expected_written, "encode len={len}");
        assert_eq!(
            &encoded[..written],
            &expected_encoded[..expected_written],
            "encode len={len}"
        );
        assert!(encoded[written..].iter().all(|byte| *byte == SENTINEL));

        let mut expected_decoded = [SENTINEL; 193];
        let expected_decoded_len = codec
            .decode_into(&expected_encoded[..expected_written], &mut expected_decoded)
            .unwrap();
        let mut decoded = [SENTINEL; 260];
        decoded[..written].copy_from_slice(&encoded[..written]);
        let decoded_len = codec.decode_in_place(&mut decoded, written).unwrap();
        assert_eq!(decoded_len, expected_decoded_len, "decode len={len}");
        assert_eq!(&decoded[..decoded_len], &input[..len], "decode len={len}");
    }
}

fn fill_pattern(bytes: &mut [u8]) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .unwrap_or(0)
            .wrapping_mul(73)
            .wrapping_add(29);
    }
}
