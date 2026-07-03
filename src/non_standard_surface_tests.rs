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
    let mut read = 0;
    while read < unwrapped_len {
        if read != 0 {
            let line_ending = wrap.line_ending().as_bytes();
            expected[output_offset..output_offset + line_ending.len()].copy_from_slice(line_ending);
            output_offset += line_ending.len();
        }
        let take = (unwrapped_len - read).min(wrap.line_len());
        expected[output_offset..output_offset + take]
            .copy_from_slice(&unwrapped[read..read + take]);
        output_offset += take;
        read += take;
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

fn assert_decode_error_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut output = [0x55; 128];
    let mut scalar_output = [0xaa; 128];

    let result = engine.decode_slice(input, &mut output);
    let scalar_result = scalar::scalar_reference_decode_slice::<A, PAD>(input, &mut scalar_output);

    assert_eq!(result, scalar_result);
}

fn assert_in_place_decode_error_matches_slice(input: &[u8]) {
    let mut in_place = [0u8; 32];
    let mut output = [0u8; 32];
    in_place[..input.len()].copy_from_slice(input);

    let in_place_result = STANDARD
        .decode_in_place(&mut in_place[..input.len()])
        .map(|decoded| decoded.len());
    let slice_result = STANDARD.decode_slice(input, &mut output);

    assert_eq!(in_place_result, slice_result);
}

fn assert_in_place_encode_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut in_place = [0xee; 256];
    let mut scalar_output = [0xaa; 256];

    in_place[..input.len()].copy_from_slice(input);
    let encoded = engine.encode_in_place(&mut in_place, input.len()).unwrap();
    let scalar_len =
        scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar_output).unwrap();

    assert_eq!(encoded, &scalar_output[..scalar_len]);
}

fn assert_slice_clear_tail_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut encoded = [0x55; 256];
    let mut scalar_encoded = [0xaa; 256];
    let mut decoded = [0x33; 128];
    let mut scalar_decoded = [0x77; 128];

    let encoded_len = engine.encode_slice_clear_tail(input, &mut encoded).unwrap();
    let scalar_encoded_len =
        scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar_encoded).unwrap();

    assert_eq!(encoded_len, scalar_encoded_len);
    assert_eq!(
        &encoded[..encoded_len],
        &scalar_encoded[..scalar_encoded_len]
    );
    assert!(encoded[encoded_len..].iter().all(|byte| *byte == 0));

    let decoded_len = engine
        .decode_slice_clear_tail(&encoded[..encoded_len], &mut decoded)
        .unwrap();
    let scalar_decoded_len = scalar::scalar_reference_decode_slice::<A, PAD>(
        &scalar_encoded[..scalar_encoded_len],
        &mut scalar_decoded,
    )
    .unwrap();

    assert_eq!(decoded_len, scalar_decoded_len);
    assert_eq!(
        &decoded[..decoded_len],
        &scalar_decoded[..scalar_decoded_len]
    );
    assert_eq!(&decoded[..decoded_len], input);
    assert!(decoded[decoded_len..].iter().all(|byte| *byte == 0));
}

fn assert_wrapped_profile_matches_engine(
    profile: &Profile<Standard, true>,
    wrap: LineWrap,
    input: &[u8],
) {
    let mut profile_encoded = [0x55; 256];
    let mut engine_encoded = [0xaa; 256];
    let mut profile_decoded = [0x33; 128];
    let mut engine_decoded = [0x77; 128];

    assert_eq!(
        profile.encoded_len(input.len()),
        STANDARD.wrapped_encoded_len(input.len(), wrap)
    );
    let profile_encoded_len = profile.encode_slice(input, &mut profile_encoded).unwrap();
    let engine_encoded_len = STANDARD
        .encode_slice_wrapped(input, &mut engine_encoded, wrap)
        .unwrap();

    assert_eq!(profile_encoded_len, engine_encoded_len);
    assert_eq!(
        &profile_encoded[..profile_encoded_len],
        &engine_encoded[..engine_encoded_len]
    );
    assert_eq!(
        profile.validate_result(&profile_encoded[..profile_encoded_len]),
        STANDARD.validate_wrapped_result(&engine_encoded[..engine_encoded_len], wrap)
    );

    let profile_decoded_len = profile
        .decode_slice_clear_tail(
            &profile_encoded[..profile_encoded_len],
            &mut profile_decoded,
        )
        .unwrap();
    let engine_decoded_len = STANDARD
        .decode_slice_wrapped_clear_tail(
            &engine_encoded[..engine_encoded_len],
            &mut engine_decoded,
            wrap,
        )
        .unwrap();

    assert_eq!(profile_decoded_len, engine_decoded_len);
    assert_eq!(
        &profile_decoded[..profile_decoded_len],
        &engine_decoded[..engine_decoded_len]
    );
    assert_eq!(&profile_decoded[..profile_decoded_len], input);
    assert!(
        profile_decoded[profile_decoded_len..]
            .iter()
            .all(|byte| *byte == 0)
    );

    let mut profile_in_place = [0u8; 256];
    let mut engine_in_place = [0u8; 256];
    profile_in_place[..profile_encoded_len]
        .copy_from_slice(&profile_encoded[..profile_encoded_len]);
    engine_in_place[..engine_encoded_len].copy_from_slice(&engine_encoded[..engine_encoded_len]);

    let profile_in_place_len = profile
        .decode_in_place_clear_tail(&mut profile_in_place[..profile_encoded_len])
        .unwrap()
        .len();
    let engine_in_place_len = STANDARD
        .decode_in_place_wrapped_clear_tail(&mut engine_in_place[..engine_encoded_len], wrap)
        .unwrap()
        .len();

    assert_eq!(profile_in_place_len, engine_in_place_len);
    assert_eq!(
        &profile_in_place[..profile_in_place_len],
        &engine_in_place[..engine_in_place_len]
    );
}

fn assert_unwrapped_profile_matches_engine<A, const PAD: bool>(
    profile: &Profile<A, PAD>,
    engine: Engine<A, PAD>,
    input: &[u8],
) where
    A: Alphabet,
{
    let mut profile_encoded = [0x55; 256];
    let mut engine_encoded = [0xaa; 256];
    let mut profile_decoded = [0x33; 128];
    let mut engine_decoded = [0x77; 128];

    assert_eq!(
        profile.encoded_len(input.len()),
        engine.encoded_len(input.len())
    );
    let profile_encoded_len = profile.encode_slice(input, &mut profile_encoded).unwrap();
    let engine_encoded_len = engine.encode_slice(input, &mut engine_encoded).unwrap();

    assert_eq!(profile_encoded_len, engine_encoded_len);
    assert_eq!(
        &profile_encoded[..profile_encoded_len],
        &engine_encoded[..engine_encoded_len]
    );
    assert_eq!(
        profile.validate_result(&profile_encoded[..profile_encoded_len]),
        engine.validate_result(&engine_encoded[..engine_encoded_len])
    );

    let profile_decoded_len = profile
        .decode_slice_clear_tail(
            &profile_encoded[..profile_encoded_len],
            &mut profile_decoded,
        )
        .unwrap();
    let engine_decoded_len = engine
        .decode_slice_clear_tail(&engine_encoded[..engine_encoded_len], &mut engine_decoded)
        .unwrap();

    assert_eq!(profile_decoded_len, engine_decoded_len);
    assert_eq!(
        &profile_decoded[..profile_decoded_len],
        &engine_decoded[..engine_decoded_len]
    );
    assert_eq!(&profile_decoded[..profile_decoded_len], input);
    assert!(
        profile_decoded[profile_decoded_len..]
            .iter()
            .all(|byte| *byte == 0)
    );
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

#[test]
fn non_standard_simd_candidate_error_surfaces_preserve_scalar_behavior() {
    assert_decode_error_matches_scalar::<DispatchFallbackAlphabet, true>(b"++++");
    assert_decode_error_matches_scalar::<DispatchFallbackAlphabet, false>(b"++++");
    assert_decode_error_matches_scalar::<Bcrypt, false>(b"....=");
    assert_decode_error_matches_scalar::<Crypt, false>(b"!!!!");
    assert_in_place_decode_error_matches_slice(b"aGV!");

    let mut legacy = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice_legacy(b" aG\nV!", &mut legacy),
        Err(DecodeError::InvalidByte {
            index: 5,
            byte: b'!'
        })
    );

    let wrap = LineWrap::new(4, LineEnding::Lf);
    let mut wrapped_decode = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice_wrapped(b"aG\nVsbG8=", &mut wrapped_decode, wrap),
        Err(DecodeError::InvalidLineWrap { index: 2 })
    );

    let mut wrapped_encode = [0u8; 8];
    assert_eq!(
        STANDARD.encode_slice_wrapped(b"hello", &mut wrapped_encode, wrap),
        Err(EncodeError::OutputTooSmall {
            required: 9,
            available: 8
        })
    );
}

#[test]
fn non_standard_simd_candidate_clear_tail_surfaces_preserve_scalar_behavior() {
    let mut input = [0; 81];
    fill_pattern(&mut input, 29);

    assert_slice_clear_tail_matches_scalar::<DispatchFallbackAlphabet, true>(&input);
    assert_slice_clear_tail_matches_scalar::<DispatchFallbackAlphabet, false>(&input);
    assert_slice_clear_tail_matches_scalar::<Bcrypt, false>(&input);
    assert_slice_clear_tail_matches_scalar::<Crypt, false>(&input);
    assert_in_place_encode_matches_scalar::<DispatchFallbackAlphabet, true>(&input);
    assert_in_place_encode_matches_scalar::<Bcrypt, false>(&input);

    let wrap = LineWrap::new(12, LineEnding::CrLf);
    let mut wrapped_output = [0xee; 192];
    let wrapped_len = STANDARD
        .encode_slice_wrapped_clear_tail(&input, &mut wrapped_output, wrap)
        .unwrap();
    assert!(wrapped_output[wrapped_len..].iter().all(|byte| *byte == 0));

    let mut wrapped_input = [0u8; 192];
    let wrapped_input_len = STANDARD
        .encode_slice_wrapped(&input, &mut wrapped_input, wrap)
        .unwrap();
    let mut wrapped_decoded = [0xdd; 128];
    let decoded_len = STANDARD
        .decode_slice_wrapped_clear_tail(
            &wrapped_input[..wrapped_input_len],
            &mut wrapped_decoded,
            wrap,
        )
        .unwrap();

    assert_eq!(&wrapped_decoded[..decoded_len], input);
    assert!(wrapped_decoded[decoded_len..].iter().all(|byte| *byte == 0));
}

#[test]
fn non_standard_profile_surfaces_preserve_engine_routing() {
    let mut input = [0; 58];
    fill_pattern(&mut input, 41);

    assert_wrapped_profile_matches_engine(&MIME, LineWrap::MIME, &input);
    assert_wrapped_profile_matches_engine(&PEM, LineWrap::PEM, &input);
    assert_wrapped_profile_matches_engine(&PEM_CRLF, LineWrap::PEM_CRLF, &input);
    assert_unwrapped_profile_matches_engine(&BCRYPT, BCRYPT_NO_PAD, &input);
    assert_unwrapped_profile_matches_engine(&CRYPT, CRYPT_NO_PAD, &input);
}
