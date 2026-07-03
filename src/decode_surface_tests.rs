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
