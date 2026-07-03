use super::*;

struct DispatchFallbackAlphabet;

impl Alphabet for DispatchFallbackAlphabet {
    const ENCODE: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

fn fill_pattern(output: &mut [u8], seed: usize) {
    for (index, byte) in output.iter_mut().enumerate() {
        let value = (index * 73 + seed * 19) % 256;
        *byte = u8::try_from(value).unwrap();
    }
}

fn assert_backend_round_trip_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut encoded = [0x55; 256];
    let mut scalar_encoded = [0xaa; 256];
    let mut decoded = [0x33; 128];
    let mut scalar_decoded = [0x77; 128];

    let encoded_result = engine.encode_slice(input, &mut encoded);
    let scalar_encoded_result =
        scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar_encoded);

    assert_eq!(encoded_result, scalar_encoded_result);
    let written = encoded_result.unwrap();
    assert_eq!(&encoded[..written], &scalar_encoded[..written]);

    let decoded_result = engine.decode_slice(&encoded[..written], &mut decoded);
    let scalar_decoded_result =
        scalar::scalar_reference_decode_slice::<A, PAD>(&encoded[..written], &mut scalar_decoded);

    assert_eq!(decoded_result, scalar_decoded_result);
    let decoded_len = decoded_result.unwrap();
    assert_eq!(&decoded[..decoded_len], &scalar_decoded[..decoded_len]);
    assert_eq!(&decoded[..decoded_len], input);
}

fn assert_wrapped_encode_matches_unwrapped_then_wrap(input: &[u8], wrap: LineWrap) {
    let mut wrapped = [0x55; 256];
    let mut unwrapped = [0xaa; 256];
    let mut expected = [0xcc; 256];

    let wrapped_len = STANDARD
        .encode_slice_wrapped(input, &mut wrapped, wrap)
        .unwrap();
    let unwrapped_len =
        scalar::scalar_reference_encode_slice::<Standard, true>(input, &mut unwrapped).unwrap();

    let mut output_offset = 0;
    let mut column = 0;
    for byte in &unwrapped[..unwrapped_len] {
        write_wrapped_byte(*byte, &mut expected, &mut output_offset, &mut column, wrap).unwrap();
    }

    assert_eq!(wrapped_len, output_offset);
    assert_eq!(&wrapped[..wrapped_len], &expected[..output_offset]);
}

fn assert_legacy_decode_matches_strict_after_compaction(input: &[u8], compacted: &[u8]) {
    let mut legacy = [0x55; 128];
    let mut strict = [0xaa; 128];

    let legacy_result = STANDARD.decode_slice_legacy(input, &mut legacy);
    let strict_result =
        scalar::scalar_reference_decode_slice::<Standard, true>(compacted, &mut strict);

    assert_eq!(legacy_result, strict_result);
    if let Ok(written) = legacy_result {
        assert_eq!(&legacy[..written], &strict[..written]);
    }
}

fn assert_wrapped_decode_matches_strict_after_compaction(
    input: &[u8],
    compacted: &[u8],
    wrap: LineWrap,
) {
    let mut wrapped = [0x55; 128];
    let mut strict = [0xaa; 128];

    let wrapped_result = STANDARD.decode_slice_wrapped(input, &mut wrapped, wrap);
    let strict_result =
        scalar::scalar_reference_decode_slice::<Standard, true>(compacted, &mut strict);

    assert_eq!(wrapped_result, strict_result);
    if let Ok(written) = wrapped_result {
        assert_eq!(&wrapped[..written], &strict[..written]);
    }
}

#[test]
fn non_standard_simd_candidate_surfaces_preserve_scalar_behavior() {
    let mut input = [0; 96];
    fill_pattern(&mut input, 17);

    assert_backend_round_trip_matches_scalar::<DispatchFallbackAlphabet, true>(&input);
    assert_backend_round_trip_matches_scalar::<DispatchFallbackAlphabet, false>(&input);
    assert_backend_round_trip_matches_scalar::<Bcrypt, false>(&input);
    assert_backend_round_trip_matches_scalar::<Crypt, false>(&input);

    let mut encoded = [0; 160];
    let encoded_len = STANDARD.encode_slice(&input, &mut encoded).unwrap();
    let encoded = &encoded[..encoded_len];

    let mut in_place = [0; 160];
    in_place[..encoded.len()].copy_from_slice(encoded);
    let decoded = STANDARD
        .decode_in_place(&mut in_place[..encoded.len()])
        .unwrap();
    assert_eq!(decoded, input);

    let mut legacy = [0; 192];
    let mut legacy_len = 0;
    for (index, byte) in encoded.iter().enumerate() {
        if index % 7 == 0 {
            legacy[legacy_len] = b' ';
            legacy_len += 1;
        }
        legacy[legacy_len] = *byte;
        legacy_len += 1;
        if index % 11 == 10 {
            legacy[legacy_len] = b'\n';
            legacy_len += 1;
        }
    }
    assert_legacy_decode_matches_strict_after_compaction(&legacy[..legacy_len], encoded);

    let wrap = LineWrap::new(16, LineEnding::Lf);
    let mut wrapped = [0; 192];
    let wrapped_len = STANDARD
        .encode_slice_wrapped(&input, &mut wrapped, wrap)
        .unwrap();
    assert_wrapped_decode_matches_strict_after_compaction(&wrapped[..wrapped_len], encoded, wrap);
    assert_wrapped_encode_matches_unwrapped_then_wrap(&input, wrap);
}
