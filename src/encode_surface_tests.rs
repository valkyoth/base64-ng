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
