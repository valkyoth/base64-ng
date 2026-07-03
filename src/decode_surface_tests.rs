use super::*;

fn fill_pattern(output: &mut [u8], seed: usize) {
    for (index, byte) in output.iter_mut().enumerate() {
        let value = (index * 73 + seed * 19) % 256;
        *byte = u8::try_from(value).unwrap();
    }
}

fn assert_standard_family_decode_surface_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut encoded = [0u8; 512];
    let encoded_len = scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut encoded).unwrap();
    let encoded = &encoded[..encoded_len];

    let mut slice_output = [0x55; 512];
    let mut clear_tail_output = [0xaa; 512];
    let mut scalar_output = [0xcc; 512];

    let scalar_result =
        scalar::scalar_reference_decode_slice::<A, PAD>(encoded, &mut scalar_output);
    let slice_result = engine.decode_slice(encoded, &mut slice_output);
    let clear_tail_result = engine.decode_slice_clear_tail(encoded, &mut clear_tail_output);
    let buffer = engine.decode_buffer::<512>(encoded).unwrap();

    assert_eq!(scalar_result, Ok(input.len()));
    assert_eq!(slice_result, scalar_result);
    assert_eq!(clear_tail_result, scalar_result);
    assert_eq!(&slice_output[..input.len()], input);
    assert_eq!(&slice_output[..input.len()], &scalar_output[..input.len()]);
    assert_eq!(&clear_tail_output[..input.len()], input);
    assert_eq!(
        &clear_tail_output[..input.len()],
        &scalar_output[..input.len()]
    );
    assert!(
        clear_tail_output[input.len()..]
            .iter()
            .all(|byte| *byte == 0)
    );
    assert_eq!(buffer.as_bytes(), input);
    assert_eq!(buffer.as_bytes(), &scalar_output[..input.len()]);

    #[cfg(feature = "alloc")]
    {
        let decoded_vec = engine.decode_vec(encoded).unwrap();
        let decoded_secret = engine.decode_secret(encoded).unwrap();

        assert_eq!(decoded_vec, input);
        assert_eq!(decoded_secret.expose_secret(), input);
    }
}

fn assert_standard_family_decode_error_surface_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut slice_output = [0x55; 512];
    let mut clear_tail_output = [0xaa; 512];
    let mut scalar_output = [0xcc; 512];

    let scalar_err = scalar::scalar_reference_decode_slice::<A, PAD>(input, &mut scalar_output)
        .expect_err("malformed fixture must fail through the scalar reference");
    let slice_err = engine
        .decode_slice(input, &mut slice_output)
        .expect_err("malformed fixture must fail through decode_slice");
    let clear_tail_err = engine
        .decode_slice_clear_tail(input, &mut clear_tail_output)
        .expect_err("malformed fixture must fail through decode_slice_clear_tail");
    let buffer_err = engine
        .decode_buffer::<512>(input)
        .expect_err("malformed fixture must fail through decode_buffer");

    assert_eq!(slice_err, scalar_err);
    assert_eq!(clear_tail_err, scalar_err);
    assert_eq!(buffer_err, scalar_err);
    assert!(
        clear_tail_output.iter().all(|byte| *byte == 0),
        "decode_slice_clear_tail must wipe the caller buffer on error"
    );

    #[cfg(feature = "alloc")]
    {
        let vec_err = engine
            .decode_vec(input)
            .expect_err("malformed fixture must fail through decode_vec");
        let secret_err = engine
            .decode_secret(input)
            .expect_err("malformed fixture must fail through decode_secret");

        assert_eq!(vec_err, scalar_err);
        assert_eq!(secret_err, scalar_err);
    }
}

#[test]
fn standard_family_decode_surfaces_cover_tails_and_padding() {
    let mut input = [0; 193];

    for input_len in 0..=input.len() {
        fill_pattern(&mut input[..input_len], input_len);
        let input = &input[..input_len];

        assert_standard_family_decode_surface_matches_scalar::<Standard, true>(input);
        assert_standard_family_decode_surface_matches_scalar::<Standard, false>(input);
        assert_standard_family_decode_surface_matches_scalar::<UrlSafe, true>(input);
        assert_standard_family_decode_surface_matches_scalar::<UrlSafe, false>(input);
    }
}

#[test]
fn standard_family_decode_error_surfaces_match_scalar() {
    for input in [
        b"!!!!".as_slice(),
        b"AAAA!",
        b"AA=A",
        b"Zh==",
        b"Zm9v$g==",
        b"AAAA====",
    ] {
        assert_standard_family_decode_error_surface_matches_scalar::<Standard, true>(input);
    }

    for input in [b"Z".as_slice(), b"Zg=", b"Zg==", b"Zm9v$g", b"Zh"] {
        assert_standard_family_decode_error_surface_matches_scalar::<Standard, false>(input);
    }

    for input in [b"AA+A".as_slice(), b"AA/A", b"AA=A", b"Zh==", b"AAAA!"] {
        assert_standard_family_decode_error_surface_matches_scalar::<UrlSafe, true>(input);
    }

    for input in [b"Z".as_slice(), b"AA+A", b"AA/A", b"Zg=", b"Zh"] {
        assert_standard_family_decode_error_surface_matches_scalar::<UrlSafe, false>(input);
    }
}
