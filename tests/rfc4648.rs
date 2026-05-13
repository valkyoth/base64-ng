use base64_ng::{
    DecodeError, EncodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    checked_encoded_len, decoded_capacity, decoded_len,
};

#[test]
fn rfc4648_standard_round_trips() {
    let cases: &[&[u8]] = &[
        b"",
        b"f",
        b"fo",
        b"foo",
        b"foob",
        b"fooba",
        b"foobar",
        b"The quick brown fox jumps over the lazy dog",
    ];

    for case in cases {
        let mut encoded = [0u8; 128];
        let encoded_len = STANDARD.encode_slice(case, &mut encoded).unwrap();
        let mut decoded = [0u8; 128];
        let decoded_len = STANDARD
            .decode_slice(&encoded[..encoded_len], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], *case);
    }
}

#[test]
fn unpadded_round_trips() {
    for input_len in 0..64 {
        let mut input = [0u8; 64];
        for (index, byte) in input.iter_mut().enumerate() {
            *byte = (index * 37 + input_len) as u8;
        }

        let mut encoded = [0u8; 128];
        let encoded_len = STANDARD_NO_PAD
            .encode_slice(&input[..input_len], &mut encoded)
            .unwrap();
        assert!(!encoded[..encoded_len].contains(&b'='));

        let mut decoded = [0u8; 64];
        let decoded_len = STANDARD_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], &input[..input_len]);
    }
}

#[test]
fn url_safe_alphabet_is_distinct() {
    let mut padded = [0u8; 8];
    let padded_len = URL_SAFE.encode_slice(b"\xfb\xff", &mut padded).unwrap();
    assert_eq!(&padded[..padded_len], b"-_8=");

    let mut unpadded = [0u8; 8];
    let unpadded_len = URL_SAFE_NO_PAD
        .encode_slice(b"\xfb\xff", &mut unpadded)
        .unwrap();
    assert_eq!(&unpadded[..unpadded_len], b"-_8");
}

#[test]
fn decoded_capacity_is_upper_bound() {
    for encoded_len in 0..128 {
        assert!(decoded_capacity(encoded_len) <= encoded_len / 4 * 3 + 2);
    }
}

#[test]
fn decoded_len_reports_exact_lengths() {
    assert_eq!(decoded_len(b"", true), Ok(0));
    assert_eq!(decoded_len(b"Zg==", true), Ok(1));
    assert_eq!(decoded_len(b"Zm8=", true), Ok(2));
    assert_eq!(decoded_len(b"Zm9v", true), Ok(3));
    assert_eq!(decoded_len(b"Zg", false), Ok(1));
    assert_eq!(decoded_len(b"Zm8", false), Ok(2));
    assert_eq!(decoded_len(b"Zm9v", false), Ok(3));
    assert_eq!(STANDARD.decoded_len(b"Zm9v"), Ok(3));
    assert_eq!(STANDARD_NO_PAD.decoded_len(b"Zm9v"), Ok(3));
}

#[test]
fn decoded_len_rejects_bad_lengths_and_padding() {
    assert_eq!(decoded_len(b"Z", true), Err(DecodeError::InvalidLength));
    assert_eq!(decoded_len(b"Z", false), Err(DecodeError::InvalidLength));
    assert_eq!(
        decoded_len(b"Zm=9", true),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        decoded_len(b"Zm8=", false),
        Err(DecodeError::InvalidPadding { index: 3 })
    );
}

#[test]
fn checked_encoded_len_reports_overflow() {
    assert_eq!(checked_encoded_len(usize::MAX, true), None);
    assert_eq!(checked_encoded_len(usize::MAX, false), None);
    assert_eq!(STANDARD.checked_encoded_len(usize::MAX), None);
    assert_eq!(STANDARD_NO_PAD.checked_encoded_len(usize::MAX), None);
}

#[test]
fn encode_slice_reports_small_outputs() {
    let mut output = [0u8; 1];
    assert_eq!(
        STANDARD.encode_slice(b"hi", &mut output),
        Err(EncodeError::OutputTooSmall {
            required: 4,
            available: 1,
        })
    );
}

#[test]
fn reports_absolute_invalid_byte_indexes() {
    let mut output = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice(b"Zm9v$g==", &mut output),
        Err(DecodeError::InvalidByte {
            index: 4,
            byte: b'$',
        })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zm9vYg$", &mut output),
        Err(DecodeError::InvalidByte {
            index: 6,
            byte: b'$',
        })
    );

    let mut input = *b"Zm9vYg$";
    assert_eq!(
        STANDARD_NO_PAD.decode_in_place(&mut input),
        Err(DecodeError::InvalidByte {
            index: 6,
            byte: b'$',
        })
    );
}

#[test]
fn reports_absolute_padding_indexes() {
    let mut output = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice(b"Zm9vZh==", &mut output),
        Err(DecodeError::InvalidPadding { index: 5 })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zm9vZh", &mut output),
        Err(DecodeError::InvalidPadding { index: 5 })
    );

    let mut input = *b"Zm9vZh";
    assert_eq!(
        STANDARD_NO_PAD.decode_in_place(&mut input),
        Err(DecodeError::InvalidPadding { index: 5 })
    );
}

#[cfg(feature = "alloc")]
#[test]
fn alloc_helpers_round_trip() {
    let encoded = STANDARD.encode_vec(b"hello").unwrap();
    assert_eq!(encoded, b"aGVsbG8=");

    let decoded = STANDARD.decode_vec(&encoded).unwrap();
    assert_eq!(decoded, b"hello");

    assert_eq!(
        STANDARD_NO_PAD.decode_vec(b"Zm8=").unwrap_err(),
        DecodeError::InvalidPadding { index: 3 }
    );
}
