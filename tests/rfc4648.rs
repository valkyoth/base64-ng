use base64_ng::{
    DecodeError, EncodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    checked_encoded_len, decoded_capacity, decoded_len,
};

#[cfg(feature = "stream")]
use base64_ng::stream::{Encoder, EncoderReader};

#[cfg(feature = "stream")]
use std::io::{Read, Write};

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
        decoded_len(b"Z=m9", true),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
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
fn encodes_in_place() {
    let mut standard = [0u8; 8];
    standard[..5].copy_from_slice(b"hello");
    let encoded = STANDARD.encode_in_place(&mut standard, 5).unwrap();
    assert_eq!(encoded, b"aGVsbG8=");

    let mut standard_no_pad = [0u8; 7];
    standard_no_pad[..5].copy_from_slice(b"hello");
    let encoded = STANDARD_NO_PAD
        .encode_in_place(&mut standard_no_pad, 5)
        .unwrap();
    assert_eq!(encoded, b"aGVsbG8");

    let mut url_safe = [0u8; 4];
    url_safe[..2].copy_from_slice(b"\xfb\xff");
    let encoded = URL_SAFE.encode_in_place(&mut url_safe, 2).unwrap();
    assert_eq!(encoded, b"-_8=");

    let mut url_safe_no_pad = [0u8; 3];
    url_safe_no_pad[..2].copy_from_slice(b"\xfb\xff");
    let encoded = URL_SAFE_NO_PAD
        .encode_in_place(&mut url_safe_no_pad, 2)
        .unwrap();
    assert_eq!(encoded, b"-_8");
}

#[test]
fn encode_in_place_reports_bad_lengths() {
    let mut too_small = [0u8; 3];
    too_small[..2].copy_from_slice(b"hi");
    assert_eq!(
        STANDARD.encode_in_place(&mut too_small, 2),
        Err(EncodeError::OutputTooSmall {
            required: 4,
            available: 3,
        })
    );

    let mut buffer = [0u8; 2];
    assert_eq!(
        STANDARD.encode_in_place(&mut buffer, 3),
        Err(EncodeError::InputTooLarge {
            input_len: 3,
            buffer_len: 2,
        })
    );
}

#[test]
fn exhaustive_short_round_trips() {
    for b0 in u8::MIN..=u8::MAX {
        let input = [b0];
        assert_round_trip(&STANDARD, &input);
        assert_round_trip(&STANDARD_NO_PAD, &input);
        assert_round_trip(&URL_SAFE, &input);
        assert_round_trip(&URL_SAFE_NO_PAD, &input);
        assert_in_place_encode_matches_slice(&STANDARD, &input);
        assert_in_place_encode_matches_slice(&STANDARD_NO_PAD, &input);
        assert_in_place_encode_matches_slice(&URL_SAFE, &input);
        assert_in_place_encode_matches_slice(&URL_SAFE_NO_PAD, &input);
    }

    for b0 in u8::MIN..=u8::MAX {
        for b1 in u8::MIN..=u8::MAX {
            let input = [b0, b1];
            assert_round_trip(&STANDARD, &input);
            assert_round_trip(&STANDARD_NO_PAD, &input);
            assert_round_trip(&URL_SAFE, &input);
            assert_round_trip(&URL_SAFE_NO_PAD, &input);
            assert_in_place_encode_matches_slice(&STANDARD, &input);
            assert_in_place_encode_matches_slice(&STANDARD_NO_PAD, &input);
            assert_in_place_encode_matches_slice(&URL_SAFE, &input);
            assert_in_place_encode_matches_slice(&URL_SAFE_NO_PAD, &input);
        }
    }
}

#[test]
fn rejects_common_non_alphabet_bytes() {
    let mut output = [0u8; 4];
    for byte in [b' ', b'\n', b'\r', b'\t', 0, 0xff] {
        let input = [b'A', b'A', byte, b'A'];
        assert_eq!(
            STANDARD.decode_slice(&input, &mut output),
            Err(DecodeError::InvalidByte { index: 2, byte })
        );
    }
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

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_handles_chunk_boundaries() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    encoder.write_all(b"h").unwrap();
    encoder.write_all(b"el").unwrap();
    encoder.write_all(b"lo").unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"aGVsbG8=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_supports_no_padding() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD_NO_PAD);
    encoder.write_all(b"he").unwrap();
    encoder.write_all(b"llo").unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"aGVsbG8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_exposes_inner_writer() {
    let mut encoder = Encoder::new(Vec::new(), URL_SAFE_NO_PAD);
    assert!(encoder.get_ref().is_empty());
    encoder.write_all(b"\xfb\xff").unwrap();
    assert!(encoder.get_ref().is_empty());
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"-_8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_handles_small_reads() {
    let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
    let mut output = [0u8; 8];
    let mut written = 0;
    while written < output.len() {
        let read = reader.read(&mut output[written..written + 1]).unwrap();
        if read == 0 {
            break;
        }
        written += read;
    }
    assert_eq!(&output[..written], b"aGVsbG8=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_supports_no_padding() {
    let mut reader = EncoderReader::new(&b"hello"[..], STANDARD_NO_PAD);
    let mut encoded = Vec::new();
    reader.read_to_end(&mut encoded).unwrap();
    assert_eq!(encoded, b"aGVsbG8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_supports_url_safe() {
    let mut reader = EncoderReader::new(&b"\xfb\xff"[..], URL_SAFE_NO_PAD);
    let mut encoded = Vec::new();
    assert_eq!(reader.get_ref().len(), 2);
    reader.read_to_end(&mut encoded).unwrap();
    assert_eq!(encoded, b"-_8");
}

fn assert_round_trip<A, const PAD: bool>(engine: &base64_ng::Engine<A, PAD>, input: &[u8])
where
    A: base64_ng::Alphabet,
{
    let mut encoded = [0u8; 4];
    let encoded_len = engine.encode_slice(input, &mut encoded).unwrap();
    let mut decoded = [0u8; 2];
    let decoded_len = engine
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .unwrap();
    assert_eq!(&decoded[..decoded_len], input);
}

fn assert_in_place_encode_matches_slice<A, const PAD: bool>(
    engine: &base64_ng::Engine<A, PAD>,
    input: &[u8],
) where
    A: base64_ng::Alphabet,
{
    let required = engine.encoded_len(input.len());
    let mut expected = [0u8; 4];
    let expected_len = engine.encode_slice(input, &mut expected).unwrap();
    assert_eq!(required, expected_len);

    let mut buffer = [0u8; 4];
    buffer[..input.len()].copy_from_slice(input);
    let encoded = engine
        .encode_in_place(&mut buffer[..required], input.len())
        .unwrap();
    assert_eq!(encoded, &expected[..expected_len]);
}
