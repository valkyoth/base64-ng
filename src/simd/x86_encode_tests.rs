use super::{
    avx2_available, avx512_vbmi_base64_available, encode_12_bytes_ssse3_sse41,
    encode_24_bytes_avx2, encode_48_bytes_avx512, encode_slice_avx2, encode_slice_avx512,
    encode_slice_ssse3_sse41, ssse3_sse41_available,
};
use crate::runtime::Backend;
use crate::{Alphabet, Standard, StaticBackendToken, UrlSafe, checked_encoded_len};

const MAX_INPUT: usize = 193;
const MAX_OUTPUT: usize = 260;

fn scalar_block<A, const INPUT: usize, const OUTPUT: usize>(input: &[u8; INPUT]) -> [u8; OUTPUT]
where
    A: Alphabet,
{
    let mut output = [0xa5; OUTPUT];
    let written = crate::scalar::encode_slice::<A, true>(input, &mut output).unwrap();
    assert_eq!(written, OUTPUT);
    output
}

fn verify_ssse3_byte_positions<A>()
where
    A: Alphabet,
{
    let mut input = [0xa5; 12];
    for position in 0..input.len() {
        for byte in u8::MIN..=u8::MAX {
            input[position] = byte;
            let expected = scalar_block::<A, 12, 16>(&input);
            let mut actual = [0x5a; 16];
            // SAFETY: The test checks the complete feature bundle before it
            // invokes this target-feature block primitive.
            unsafe { encode_12_bytes_ssse3_sse41::<A>(&input, &mut actual) };
            assert_eq!(actual, expected, "position={position}, byte={byte}");
        }
        input[position] = 0xa5;
    }
}

fn verify_avx2_byte_positions<A>()
where
    A: Alphabet,
{
    let mut input = [0xa5; 24];
    for position in 0..input.len() {
        for byte in u8::MIN..=u8::MAX {
            input[position] = byte;
            let expected = scalar_block::<A, 24, 32>(&input);
            let mut actual = [0x5a; 32];
            // SAFETY: The test checks AVX2 availability before it invokes
            // this target-feature block primitive.
            unsafe { encode_24_bytes_avx2::<A>(&input, &mut actual) };
            assert_eq!(actual, expected, "position={position}, byte={byte}");
        }
        input[position] = 0xa5;
    }
}

fn verify_avx512_byte_positions<A>()
where
    A: Alphabet,
{
    let mut input = [0xa5; 48];
    for position in 0..input.len() {
        for byte in u8::MIN..=u8::MAX {
            input[position] = byte;
            let expected = scalar_block::<A, 48, 64>(&input);
            let mut actual = [0x5a; 64];
            // SAFETY: The test checks the complete AVX-512 Base64 feature
            // bundle before invoking this target-feature block primitive.
            unsafe { encode_48_bytes_avx512::<A>(&input, &mut actual) };
            assert_eq!(actual, expected, "position={position}, byte={byte}");
        }
        input[position] = 0xa5;
    }
}

fn patterned_input(len: usize) -> [u8; MAX_INPUT] {
    let mut input = [0u8; MAX_INPUT];
    let mut value = len.to_le_bytes()[0];
    for byte in &mut input[..len] {
        value = value.wrapping_mul(73).wrapping_add(19);
        *byte = value;
    }
    input
}

fn verify_slice<A, const PAD: bool>(
    encode: fn(&[u8], &mut [u8]) -> Result<usize, crate::EncodeError>,
) where
    A: Alphabet,
{
    for len in 0..=MAX_INPUT {
        let input = patterned_input(len);
        let required = checked_encoded_len(len, PAD).unwrap();
        let mut expected = [0xa5; MAX_OUTPUT];
        let mut actual = [0x5a; MAX_OUTPUT];
        let expected_len =
            crate::scalar::encode_slice::<A, PAD>(&input[..len], &mut expected[..required])
                .unwrap();
        let actual_len = encode(&input[..len], &mut actual[..required]).unwrap();
        assert_eq!(actual_len, expected_len, "input length={len}");
        assert_eq!(
            &actual[..actual_len],
            &expected[..expected_len],
            "input length={len}"
        );
        assert!(
            actual[required..].iter().all(|byte| *byte == 0x5a),
            "write exceeded required output for input length={len}"
        );
    }
}

fn verify_static_token(backend: Backend) {
    // SAFETY: The caller reaches this helper only after the matching runtime
    // CPU feature check. The test thread is not migrated while the token lives.
    let token = unsafe { StaticBackendToken::assume_supported(backend) }
        .expect("available backend must pass its direct KAT");
    assert_eq!(token.backend(), backend);

    let input = patterned_input(97);
    let mut expected = [0xa5; MAX_OUTPUT];
    let mut actual = [0x5a; MAX_OUTPUT];
    let expected_len =
        crate::scalar::encode_slice::<Standard, true>(&input[..97], &mut expected).unwrap();
    let actual_len = token
        .encode_standard::<true>(&input[..97], &mut actual)
        .unwrap();
    assert_eq!(&actual[..actual_len], &expected[..expected_len]);

    let expected_len =
        crate::scalar::encode_slice::<UrlSafe, false>(&input[..97], &mut expected).unwrap();
    let actual_len = token
        .encode_url_safe::<false>(&input[..97], &mut actual)
        .unwrap();
    assert_eq!(&actual[..actual_len], &expected[..expected_len]);
}

#[test]
fn ssse3_encode_is_exhaustive_per_byte_and_matches_scalar_for_tails() {
    if !std::is_x86_feature_detected!("ssse3")
        || !std::is_x86_feature_detected!("sse4.1")
        || !ssse3_sse41_available()
    {
        return;
    }
    verify_ssse3_byte_positions::<Standard>();
    verify_ssse3_byte_positions::<UrlSafe>();
    verify_slice::<Standard, true>(encode_slice_ssse3_sse41::<Standard, true>);
    verify_slice::<Standard, false>(encode_slice_ssse3_sse41::<Standard, false>);
    verify_slice::<UrlSafe, true>(encode_slice_ssse3_sse41::<UrlSafe, true>);
    verify_slice::<UrlSafe, false>(encode_slice_ssse3_sse41::<UrlSafe, false>);
    verify_static_token(Backend::Ssse3Sse41);
}

#[test]
fn avx2_encode_is_exhaustive_per_byte_and_matches_scalar_for_tails() {
    if !std::is_x86_feature_detected!("avx2") || !avx2_available() {
        return;
    }
    verify_avx2_byte_positions::<Standard>();
    verify_avx2_byte_positions::<UrlSafe>();
    verify_slice::<Standard, true>(encode_slice_avx2::<Standard, true>);
    verify_slice::<Standard, false>(encode_slice_avx2::<Standard, false>);
    verify_slice::<UrlSafe, true>(encode_slice_avx2::<UrlSafe, true>);
    verify_slice::<UrlSafe, false>(encode_slice_avx2::<UrlSafe, false>);
    verify_static_token(Backend::Avx2);
}

#[test]
fn avx512_encode_is_exhaustive_per_byte_and_matches_scalar_for_tails() {
    if !std::is_x86_feature_detected!("avx512f")
        || !std::is_x86_feature_detected!("avx512bw")
        || !std::is_x86_feature_detected!("avx512vl")
        || !std::is_x86_feature_detected!("avx512vbmi")
        || !avx512_vbmi_base64_available()
    {
        return;
    }
    verify_avx512_byte_positions::<Standard>();
    verify_avx512_byte_positions::<UrlSafe>();
    verify_slice::<Standard, true>(encode_slice_avx512::<Standard, true>);
    verify_slice::<Standard, false>(encode_slice_avx512::<Standard, false>);
    verify_slice::<UrlSafe, true>(encode_slice_avx512::<UrlSafe, true>);
    verify_slice::<UrlSafe, false>(encode_slice_avx512::<UrlSafe, false>);
    verify_static_token(Backend::Avx512Vbmi);
}

#[test]
fn avx512_automatic_policy_uses_narrower_backends_below_crossover() {
    assert!(!crate::encode_backend::avx512_auto_preferred(191));
    assert!(crate::encode_backend::avx512_auto_preferred(192));

    if crate::encode_backend::candidate_encode_backend()
        != crate::encode_backend::EncodeBackend::Avx512Vbmi
    {
        return;
    }
    assert_eq!(
        crate::encode_backend::active_encode_backend_for_input(12),
        crate::encode_backend::EncodeBackend::Ssse3Sse41
    );
    assert_eq!(
        crate::encode_backend::active_encode_backend_for_input(48),
        crate::encode_backend::EncodeBackend::Avx2
    );
    assert_eq!(
        crate::encode_backend::active_encode_backend_for_input(192),
        crate::encode_backend::EncodeBackend::Avx512Vbmi
    );
}
