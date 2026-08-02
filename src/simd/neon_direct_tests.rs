use crate::{Alphabet, Standard, UrlSafe};

use super::neon::{test_direct_decode_16, test_direct_encode_12};
use super::{decode_slice_neon, encode_slice_neon};

#[test]
fn direct_neon_encode_is_exhaustive_by_input_byte_and_position() {
    for position in 0..12 {
        for value in u8::MIN..=u8::MAX {
            let mut input = [0u8; 12];
            fill_pattern(&mut input, value.wrapping_mul(17));
            input[position] = value;
            assert_direct_encode::<Standard>(&input);
            assert_direct_encode::<UrlSafe>(&input);
        }
    }
}

#[test]
fn direct_neon_decode_accepts_every_symbol_in_every_lane() {
    for position in 0..16 {
        for value in 0..64 {
            let mut standard = *b"QUJDREVGR0hJSktM";
            standard[position] = Standard::ENCODE[value];
            assert_direct_decode::<Standard>(&standard);

            let mut url_safe = *b"QUJDREVGR0hJSktM";
            url_safe[position] = UrlSafe::ENCODE[value];
            assert_direct_decode::<UrlSafe>(&url_safe);
        }
    }
}

#[test]
fn direct_neon_decode_rejects_every_non_alphabet_byte_without_writing() {
    for position in 0..16 {
        for byte in u8::MIN..=u8::MAX {
            if !Standard::ENCODE.contains(&byte) {
                let mut input = *b"QUJDREVGR0hJSktM";
                input[position] = byte;
                let mut output = [0x5a; 12];
                assert!(!test_direct_decode_16::<Standard>(&input, &mut output));
                assert_eq!(output, [0x5a; 12]);
            }
            if !UrlSafe::ENCODE.contains(&byte) {
                let mut input = *b"QUJDREVGR0hJSktM";
                input[position] = byte;
                let mut output = [0xa5; 12];
                assert!(!test_direct_decode_16::<UrlSafe>(&input, &mut output));
                assert_eq!(output, [0xa5; 12]);
            }
        }
    }
}

#[test]
fn neon_slice_paths_match_scalar_across_blocks_tails_and_padding() {
    for len in 0..=385 {
        let mut input = [0u8; 385];
        fill_pattern(&mut input, u8::try_from(len % 256).unwrap());
        assert_slice_case::<Standard, true>(&input[..len]);
        assert_slice_case::<Standard, false>(&input[..len]);
        assert_slice_case::<UrlSafe, true>(&input[..len]);
        assert_slice_case::<UrlSafe, false>(&input[..len]);
    }
}

#[test]
fn neon_strict_decode_preserves_scalar_errors_for_every_position() {
    let mut raw = [0u8; 192];
    fill_pattern(&mut raw, 0x91);
    let mut encoded = [0u8; 256];
    let encoded_len = crate::scalar::encode_slice::<Standard, true>(&raw, &mut encoded).unwrap();

    for position in 0..encoded_len {
        let original = encoded[position];
        encoded[position] = b'!';
        let mut neon = [0x55; 192];
        let mut scalar = [0xaa; 192];
        let neon_error =
            decode_slice_neon::<Standard, true>(&encoded[..encoded_len], &mut neon).unwrap_err();
        let scalar_error =
            crate::scalar::decode_slice::<Standard, true>(&encoded[..encoded_len], &mut scalar)
                .unwrap_err();
        assert_eq!(neon_error, scalar_error);
        assert_eq!(neon, [0x55; 192]);
        encoded[position] = original;
    }
}

fn assert_direct_encode<A: Alphabet>(input: &[u8; 12]) {
    let mut direct = [0x55; 16];
    let mut scalar = [0xaa; 16];
    assert!(test_direct_encode_12::<A>(input, &mut direct));
    let written = crate::scalar::encode_slice::<A, true>(input, &mut scalar).unwrap();
    assert_eq!(written, direct.len());
    assert_eq!(direct, scalar);
}

fn assert_direct_decode<A: Alphabet>(input: &[u8; 16]) {
    let mut direct = [0x55; 12];
    let mut scalar = [0xaa; 12];
    assert!(test_direct_decode_16::<A>(input, &mut direct));
    let written = crate::scalar::decode_slice::<A, false>(input, &mut scalar).unwrap();
    assert_eq!(written, direct.len());
    assert_eq!(direct, scalar);
}

fn assert_slice_case<A: Alphabet, const PAD: bool>(input: &[u8]) {
    let mut neon_encoded = [0x55; 516];
    let mut scalar_encoded = [0xaa; 516];
    let neon_len = encode_slice_neon::<A, PAD>(input, &mut neon_encoded).unwrap();
    let scalar_len = crate::scalar::encode_slice::<A, PAD>(input, &mut scalar_encoded).unwrap();
    assert_eq!(neon_len, scalar_len);
    assert_eq!(&neon_encoded[..neon_len], &scalar_encoded[..scalar_len]);

    let mut neon_decoded = [0x55; 385];
    let mut scalar_decoded = [0xaa; 385];
    let neon_len =
        decode_slice_neon::<A, PAD>(&neon_encoded[..neon_len], &mut neon_decoded).unwrap();
    let scalar_len =
        crate::scalar::decode_slice::<A, PAD>(&neon_encoded[..neon_len], &mut scalar_decoded)
            .unwrap();
    assert_eq!(neon_len, scalar_len);
    assert_eq!(&neon_decoded[..neon_len], input);
    assert_eq!(&neon_decoded[..neon_len], &scalar_decoded[..scalar_len]);
}

fn fill_pattern(output: &mut [u8], seed: u8) {
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = seed
            .wrapping_add(u8::try_from(index % 256).unwrap().wrapping_mul(73))
            .rotate_left(u32::try_from(index % 8).unwrap());
    }
}
