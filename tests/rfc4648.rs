use base64_ng::{
    DecodeError, EncodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    checked_encoded_len, decoded_capacity, decoded_len,
};

#[cfg(feature = "stream")]
use base64_ng::stream::{Decoder, DecoderReader, Encoder, EncoderReader};

#[cfg(feature = "stream")]
use std::io::{Read, Write};

#[cfg(feature = "stream")]
struct ChunkedReader<'a> {
    input: &'a [u8],
    max_chunk: usize,
}

#[cfg(feature = "stream")]
impl Read for ChunkedReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let len = self.input.len().min(self.max_chunk).min(output.len());
        if len == 0 {
            return Ok(0);
        }

        output[..len].copy_from_slice(&self.input[..len]);
        self.input = &self.input[len..];
        Ok(len)
    }
}

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
fn const_array_encoding_matches_runtime_encoding() {
    const STANDARD_HELLO: [u8; 8] = STANDARD.encode_array(b"hello");
    const STANDARD_HELLO_NO_PAD: [u8; 7] = STANDARD_NO_PAD.encode_array(b"hello");
    const URL_SAFE_BYTES: [u8; 4] = URL_SAFE.encode_array(b"\xfb\xff");
    const URL_SAFE_BYTES_NO_PAD: [u8; 3] = URL_SAFE_NO_PAD.encode_array(b"\xfb\xff");
    const EMPTY: [u8; 0] = STANDARD.encode_array(b"");

    assert_eq!(&STANDARD_HELLO, b"aGVsbG8=");
    assert_eq!(&STANDARD_HELLO_NO_PAD, b"aGVsbG8");
    assert_eq!(&URL_SAFE_BYTES, b"-_8=");
    assert_eq!(&URL_SAFE_BYTES_NO_PAD, b"-_8");
    assert_eq!(&EMPTY, b"");
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
fn decode_slice_reports_small_outputs() {
    let mut output = [0u8; 1];
    assert_eq!(
        STANDARD.decode_slice(b"aGk=", &mut output),
        Err(DecodeError::OutputTooSmall {
            required: 2,
            available: 1,
        })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"aGk", &mut output),
        Err(DecodeError::OutputTooSmall {
            required: 2,
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
#[cfg_attr(miri, ignore)]
fn deterministic_long_round_trips() {
    let mut input = Vec::new();
    for len in 0..=1024 {
        input.resize(len, 0);
        fill_deterministic(&mut input, len as u64);

        assert_equivalent_round_trip(&STANDARD, &input);
        assert_equivalent_round_trip(&STANDARD_NO_PAD, &input);
        assert_equivalent_round_trip(&URL_SAFE, &input);
        assert_equivalent_round_trip(&URL_SAFE_NO_PAD, &input);
    }
}

#[test]
#[cfg_attr(miri, ignore)]
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
fn rejects_mixed_alphabet_bytes() {
    let mut output = [0u8; 4];
    assert_eq!(
        STANDARD.decode_slice(b"AA-A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'-',
        })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"AA_A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'_',
        })
    );
    assert_eq!(
        URL_SAFE.decode_slice(b"AA+A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'+',
        })
    );
    assert_eq!(
        URL_SAFE_NO_PAD.decode_slice(b"AA/A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'/',
        })
    );

    let mut standard_input = *b"AA-A";
    assert_eq!(
        STANDARD.decode_in_place(&mut standard_input),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'-',
        })
    );

    let mut url_safe_input = *b"AA+A";
    assert_eq!(
        URL_SAFE.decode_in_place(&mut url_safe_input),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'+',
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

#[test]
fn rejects_non_canonical_trailing_bits() {
    let mut output = [0u8; 4];
    assert_eq!(
        STANDARD.decode_slice(b"Zh==", &mut output),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert_eq!(
        STANDARD.decode_slice(b"Zm9=", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zh", &mut output),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zm9", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        URL_SAFE.decode_slice(b"-_9=", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        URL_SAFE_NO_PAD.decode_slice(b"-_9", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );

    let mut input = *b"Zm9";
    assert_eq!(
        STANDARD_NO_PAD.decode_in_place(&mut input),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
}

#[cfg(feature = "alloc")]
#[test]
fn alloc_helpers_round_trip() {
    let encoded = STANDARD.encode_vec(b"hello").unwrap();
    assert_eq!(encoded, b"aGVsbG8=");

    let encoded_string = STANDARD.encode_string(b"hello").unwrap();
    assert_eq!(encoded_string, "aGVsbG8=");

    let url_safe_string = URL_SAFE_NO_PAD.encode_string(b"\xfb\xff").unwrap();
    assert_eq!(url_safe_string, "-_8");

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

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_handles_fragmented_sources() {
    let input = b"any carnal pleasure.";
    let source = ChunkedReader {
        input,
        max_chunk: 1,
    };
    let mut reader = EncoderReader::new(source, STANDARD);
    let mut encoded = Vec::new();
    let mut scratch = [0u8; 2];

    loop {
        let read = reader.read(&mut scratch).unwrap();
        if read == 0 {
            break;
        }
        encoded.extend_from_slice(&scratch[..read]);
    }

    assert_eq!(encoded, STANDARD.encode_vec(input).unwrap());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_handles_chunk_boundaries() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();
    decoder.write_all(b"GVs").unwrap();
    decoder.write_all(b"bG8=").unwrap();
    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_supports_no_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD_NO_PAD);
    decoder.write_all(b"aGV").unwrap();
    decoder.write_all(b"sbG8").unwrap();
    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_bad_final_pending_input() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();
    assert!(decoder.finish().is_err());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_trailing_input_after_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    let err = decoder.write_all(b"aGk=AA").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_handles_small_reads() {
    let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
    let mut output = [0u8; 5];
    let mut written = 0;
    while written < output.len() {
        let read = reader.read(&mut output[written..written + 1]).unwrap();
        if read == 0 {
            break;
        }
        written += read;
    }
    assert_eq!(&output[..written], b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_supports_no_padding() {
    let mut reader = DecoderReader::new(&b"aGVsbG8"[..], STANDARD_NO_PAD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_supports_url_safe() {
    let mut reader = DecoderReader::new(&b"-_8"[..], URL_SAFE_NO_PAD);
    let mut decoded = Vec::new();
    assert_eq!(reader.get_ref().len(), 3);
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"\xfb\xff");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_rejects_bad_final_pending_input() {
    let mut reader = DecoderReader::new(&b"a"[..], STANDARD);
    let mut decoded = Vec::new();
    assert!(reader.read_to_end(&mut decoded).is_err());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_rejects_trailing_input_after_padding() {
    let mut reader = DecoderReader::new(&b"aGk=AA"[..], STANDARD);
    let mut decoded = Vec::new();
    let err = reader.read_to_end(&mut decoded).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_handles_fragmented_sources() {
    let encoded = b"YW55IGNhcm5hbCBwbGVhc3VyZS4=";
    let source = ChunkedReader {
        input: encoded,
        max_chunk: 1,
    };
    let mut reader = DecoderReader::new(source, STANDARD);
    let mut decoded = Vec::new();
    let mut scratch = [0u8; 2];

    loop {
        let read = reader.read(&mut scratch).unwrap();
        if read == 0 {
            break;
        }
        decoded.extend_from_slice(&scratch[..read]);
    }

    assert_eq!(decoded, b"any carnal pleasure.");
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

fn assert_equivalent_round_trip<A, const PAD: bool>(
    engine: &base64_ng::Engine<A, PAD>,
    input: &[u8],
) where
    A: base64_ng::Alphabet,
{
    let encoded_len = engine.encoded_len(input.len());
    let mut encoded = vec![0u8; encoded_len];
    let written = engine.encode_slice(input, &mut encoded).unwrap();
    assert_eq!(written, encoded_len);

    let mut in_place_encode = vec![0u8; encoded_len];
    in_place_encode[..input.len()].copy_from_slice(input);
    let in_place_encoded = engine
        .encode_in_place(&mut in_place_encode, input.len())
        .unwrap();
    assert_eq!(in_place_encoded, encoded);

    let mut decoded = vec![0u8; input.len()];
    let decoded_len = engine.decode_slice(&encoded, &mut decoded).unwrap();
    assert_eq!(decoded_len, input.len());
    assert_eq!(decoded, input);

    let mut in_place_decode = encoded;
    let in_place_decoded = engine.decode_in_place(&mut in_place_decode).unwrap();
    assert_eq!(in_place_decoded, input);
}

fn fill_deterministic(output: &mut [u8], seed: u64) {
    let mut state = seed ^ 0x243f_6a88_85a3_08d3;
    for byte in output {
        state = state
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(0xbf58_476d_1ce4_e5b9);
        *byte = (state >> 56) as u8;
    }
}
