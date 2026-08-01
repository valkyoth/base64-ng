use crate::{Alphabet, Standard, UrlSafe, scalar};

#[derive(Clone, Copy)]
enum ForcedBackend {
    Ssse3Sse41,
    Avx2,
}

#[test]
fn ssse3_and_avx2_map_every_valid_symbol_in_every_lane() {
    if super::ssse3_sse41_available() {
        exhaust_valid_block::<Standard>(ForcedBackend::Ssse3Sse41, 16);
        exhaust_valid_block::<UrlSafe>(ForcedBackend::Ssse3Sse41, 16);
    }
    if super::avx2_available() {
        exhaust_valid_block::<Standard>(ForcedBackend::Avx2, 32);
        exhaust_valid_block::<UrlSafe>(ForcedBackend::Avx2, 32);
    }
}

#[test]
fn ssse3_and_avx2_reject_every_invalid_byte_in_every_lane() {
    if super::ssse3_sse41_available() {
        exhaust_invalid_block::<Standard>(ForcedBackend::Ssse3Sse41, 16);
        exhaust_invalid_block::<UrlSafe>(ForcedBackend::Ssse3Sse41, 16);
    }
    if super::avx2_available() {
        exhaust_invalid_block::<Standard>(ForcedBackend::Avx2, 32);
        exhaust_invalid_block::<UrlSafe>(ForcedBackend::Avx2, 32);
    }
}

#[test]
fn forced_ssse3_and_avx2_slices_match_scalar_for_tails_and_errors() {
    if super::ssse3_sse41_available() {
        exhaust_slice_lengths(ForcedBackend::Ssse3Sse41);
        exhaust_malformed_positions(ForcedBackend::Ssse3Sse41);
    }
    if super::avx2_available() {
        exhaust_slice_lengths(ForcedBackend::Avx2);
        exhaust_malformed_positions(ForcedBackend::Avx2);
    }
}

fn exhaust_valid_block<A: Alphabet>(backend: ForcedBackend, block_len: usize) {
    let mut input = [b'A'; 32];
    for position in 0..block_len {
        for symbol in A::ENCODE {
            input[position] = symbol;
            assert_forced_matches_scalar::<A, false>(backend, &input[..block_len]);
        }
        input[position] = b'A';
    }
}

fn exhaust_invalid_block<A: Alphabet>(backend: ForcedBackend, block_len: usize) {
    let mut input = [b'A'; 32];
    for position in 0..block_len {
        for byte in 0u8..=u8::MAX {
            if A::decode(byte).is_some() {
                continue;
            }
            input[position] = byte;
            assert_forced_error_is_transactional::<A>(backend, &input[..block_len]);
        }
        input[position] = b'A';
    }
}

fn exhaust_slice_lengths(backend: ForcedBackend) {
    let mut raw = [0u8; 257];
    for (index, byte) in raw.iter_mut().enumerate() {
        *byte = u8::try_from((index * 73 + 19) % 256).unwrap();
    }
    for len in 0..=raw.len() {
        assert_encoded_round_trip::<Standard, true>(backend, &raw[..len]);
        assert_encoded_round_trip::<Standard, false>(backend, &raw[..len]);
        assert_encoded_round_trip::<UrlSafe, true>(backend, &raw[..len]);
        assert_encoded_round_trip::<UrlSafe, false>(backend, &raw[..len]);
    }
}

fn exhaust_malformed_positions(backend: ForcedBackend) {
    let raw = [0xa5; 192];
    let mut encoded = [0u8; 256];
    let written = scalar::encode_slice::<Standard, false>(&raw, &mut encoded).unwrap();
    for position in 0..written {
        let original = encoded[position];
        encoded[position] = b'!';
        assert_forced_error_is_transactional::<Standard>(backend, &encoded[..written]);
        encoded[position] = original;
    }
}

fn assert_encoded_round_trip<A: Alphabet, const PAD: bool>(backend: ForcedBackend, raw: &[u8]) {
    let mut encoded = [0u8; 344];
    let encoded_len = scalar::encode_slice::<A, PAD>(raw, &mut encoded).unwrap();
    assert_forced_matches_scalar::<A, PAD>(backend, &encoded[..encoded_len]);
}

fn assert_forced_matches_scalar<A: Alphabet, const PAD: bool>(
    backend: ForcedBackend,
    input: &[u8],
) {
    let mut accelerated = [0x55; 257];
    let mut reference = [0xaa; 257];
    let accelerated_result = forced_decode::<A, PAD>(backend, input, &mut accelerated);
    let reference_result = scalar::decode_slice::<A, PAD>(input, &mut reference);
    assert_eq!(accelerated_result, reference_result);
    if let Ok(written) = reference_result {
        assert_eq!(&accelerated[..written], &reference[..written]);
    }
}

fn assert_forced_error_is_transactional<A: Alphabet>(backend: ForcedBackend, input: &[u8]) {
    let mut output = [0x55; 257];
    let error = forced_decode::<A, false>(backend, input, &mut output)
        .expect_err("malformed forced-backend input must be rejected");
    assert_eq!(output, [0x55; 257], "error changed caller output: {error}");
}

fn forced_decode<A: Alphabet, const PAD: bool>(
    backend: ForcedBackend,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, crate::DecodeError> {
    match backend {
        ForcedBackend::Ssse3Sse41 => super::x86::decode_slice_ssse3_sse41::<A, PAD>(input, output),
        ForcedBackend::Avx2 => super::x86::decode_slice_avx2::<A, PAD>(input, output),
    }
}
