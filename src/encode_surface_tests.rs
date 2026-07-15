use super::*;

struct DispatchFallbackAlphabet;

impl Alphabet for DispatchFallbackAlphabet {
    const ENCODE: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

struct InconsistentEncodeAlphabet;

impl Alphabet for InconsistentEncodeAlphabet {
    const ENCODE: [u8; 64] = Standard::ENCODE;

    fn encode(_value: u8) -> u8 {
        b'!'
    }

    fn decode(byte: u8) -> Option<u8> {
        Standard::decode(byte)
    }
}

fn fill_pattern(output: &mut [u8], seed: usize) {
    for (index, byte) in output.iter_mut().enumerate() {
        let value = (index * 73 + seed * 19) % 256;
        *byte = u8::try_from(value).unwrap();
    }
}

fn assert_encode_in_place_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut dispatched = [0x55; 256];
    let mut scalar = [0xaa; 256];

    dispatched[..input.len()].copy_from_slice(input);
    scalar[..input.len()].copy_from_slice(input);

    let dispatched_result = engine
        .encode_in_place(&mut dispatched, input.len())
        .map(|encoded| encoded.len());
    let scalar_result = scalar_encode_in_place::encode_in_place::<A, PAD>(&mut scalar, input.len());

    assert_eq!(dispatched_result, scalar_result);
    if let Ok(written) = dispatched_result {
        assert_eq!(&dispatched[..written], &scalar[..written]);
    }
}

fn assert_standard_family_encode_surface_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut encoded = [0x55; 512];
    let mut clear_tail = [0x66; 512];
    let mut scalar = [0xaa; 512];

    let encoded_len = engine.encode_slice(input, &mut encoded).unwrap();
    let clear_tail_len = engine
        .encode_slice_clear_tail(input, &mut clear_tail)
        .unwrap();
    let scalar_len = scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar).unwrap();

    assert_eq!(encoded_len, scalar_len);
    assert_eq!(clear_tail_len, scalar_len);
    assert_eq!(&encoded[..encoded_len], &scalar[..scalar_len]);
    assert_eq!(&clear_tail[..clear_tail_len], &scalar[..scalar_len]);
    assert!(clear_tail[clear_tail_len..].iter().all(|byte| *byte == 0));

    let stack_buffer = engine.encode_buffer::<512>(input).unwrap();
    assert_eq!(stack_buffer.as_bytes(), &scalar[..scalar_len]);

    #[cfg(feature = "alloc")]
    {
        let encoded_vec = engine.encode_vec(input).unwrap();
        let encoded_vec_infallible = engine.encode_vec_infallible(input);
        let encoded_string = engine.encode_string(input).unwrap();
        let encoded_string_infallible = engine.encode_string_infallible(input);

        assert_eq!(encoded_vec, &scalar[..scalar_len]);
        assert_eq!(encoded_vec_infallible, &scalar[..scalar_len]);
        assert_eq!(encoded_string.as_bytes(), &scalar[..scalar_len]);
        assert_eq!(encoded_string_infallible.as_bytes(), &scalar[..scalar_len]);
    }
}

#[test]
fn standard_family_encode_surfaces_cover_tails_and_padding() {
    let mut input = [0; 193];

    for input_len in 0..=input.len() {
        fill_pattern(&mut input[..input_len], input_len);
        let input = &input[..input_len];

        assert_standard_family_encode_surface_matches_scalar::<Standard, true>(input);
        assert_standard_family_encode_surface_matches_scalar::<Standard, false>(input);
        assert_standard_family_encode_surface_matches_scalar::<UrlSafe, true>(input);
        assert_standard_family_encode_surface_matches_scalar::<UrlSafe, false>(input);
    }
}

#[test]
fn encode_in_place_backend_matches_scalar_reference() {
    let mut input = [0; 128];

    for input_len in 0..=input.len() {
        fill_pattern(&mut input[..input_len], input_len);
        let input = &input[..input_len];

        assert_encode_in_place_backend_matches_scalar::<Standard, true>(input);
        assert_encode_in_place_backend_matches_scalar::<Standard, false>(input);
        assert_encode_in_place_backend_matches_scalar::<UrlSafe, true>(input);
        assert_encode_in_place_backend_matches_scalar::<UrlSafe, false>(input);
        assert_encode_in_place_backend_matches_scalar::<DispatchFallbackAlphabet, true>(input);
        assert_encode_in_place_backend_matches_scalar::<DispatchFallbackAlphabet, false>(input);
        assert_encode_in_place_backend_matches_scalar::<Bcrypt, false>(input);
        assert_encode_in_place_backend_matches_scalar::<Crypt, false>(input);
    }
}

#[test]
fn inconsistent_encode_override_fails_before_runtime_output_writes() {
    const TABLE_OUTPUT: [u8; 4] =
        Engine::<InconsistentEncodeAlphabet, true>::new().encode_array(&[0, 0, 0]);
    assert_eq!(TABLE_OUTPUT, *b"AAAA");

    let engine = Engine::<InconsistentEncodeAlphabet, true>::new();
    let input = [0u8; 49];

    for input_len in [1, 12, 49] {
        let input = &input[..input_len];
        let mut output = [0x55; 128];
        assert_eq!(
            engine.encode_slice(input, &mut output),
            Err(EncodeError::InvalidAlphabet)
        );
        assert!(output.iter().all(|byte| *byte == 0x55));

        let mut wrapped = [0x66; 160];
        assert_eq!(
            engine.encode_slice_wrapped(input, &mut wrapped, LineWrap::new(16, LineEnding::CrLf),),
            Err(EncodeError::InvalidAlphabet)
        );
        assert!(wrapped.iter().all(|byte| *byte == 0x66));

        let mut clear_tail = [0x77; 128];
        assert_eq!(
            engine.encode_slice_clear_tail(input, &mut clear_tail),
            Err(EncodeError::InvalidAlphabet)
        );
        assert!(clear_tail.iter().all(|byte| *byte == 0));

        let mut in_place = [0x88; 128];
        in_place[..input_len].copy_from_slice(input);
        let original = in_place;
        assert_eq!(
            engine
                .encode_in_place(&mut in_place, input_len)
                .unwrap_err(),
            EncodeError::InvalidAlphabet
        );
        assert_eq!(in_place, original);

        assert_eq!(
            engine.encode_buffer::<128>(input).unwrap_err(),
            EncodeError::InvalidAlphabet
        );

        #[cfg(feature = "alloc")]
        {
            assert_eq!(engine.encode_vec(input), Err(EncodeError::InvalidAlphabet));
            assert_eq!(
                engine.encode_string(input),
                Err(EncodeError::InvalidAlphabet)
            );
        }
    }
}

#[test]
fn encode_simd_admission_rejects_non_standard_alphabet_families() {
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    for supports_alphabet in [
        simd::avx512_supports_alphabet::<DispatchFallbackAlphabet>,
        simd::avx512_supports_alphabet::<Bcrypt>,
        simd::avx512_supports_alphabet::<Crypt>,
        simd::avx2_supports_alphabet::<DispatchFallbackAlphabet>,
        simd::avx2_supports_alphabet::<Bcrypt>,
        simd::avx2_supports_alphabet::<Crypt>,
        simd::ssse3_sse41_supports_alphabet::<DispatchFallbackAlphabet>,
        simd::ssse3_sse41_supports_alphabet::<Bcrypt>,
        simd::ssse3_sse41_supports_alphabet::<Crypt>,
    ] {
        assert!(!supports_alphabet());
    }

    #[cfg(all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ))]
    for supports_alphabet in [
        simd::neon_supports_alphabet::<DispatchFallbackAlphabet>,
        simd::neon_supports_alphabet::<Bcrypt>,
        simd::neon_supports_alphabet::<Crypt>,
    ] {
        assert!(!supports_alphabet());
    }
}
