use super::*;
use crate::{Alphabet, DecodeError, Engine, Standard, UrlSafe};

fn fill_pattern(output: &mut [u8], seed: usize) {
    for (index, byte) in output.iter_mut().enumerate() {
        let value = (index * 73 + seed * 19) % 256;
        *byte = u8::try_from(value).unwrap();
    }
}

#[test]
fn ssse3_sse41_decode_block_matches_scalar_when_available() {
    if !ssse3_sse41_available() {
        println!("skipped: SSSE3/SSE4.1 decode block test requires ssse3 and sse4.1");
        return;
    }

    let mut raw = [0; 12];
    for seed in 0..64 {
        fill_pattern(&mut raw, seed);
        assert_ssse3_decode_case::<Standard, true>(&encoded_block::<Standard, true>(&raw));
        assert_ssse3_decode_case::<UrlSafe, true>(&encoded_block::<UrlSafe, true>(&raw));
        assert_ssse3_decode_case::<Standard, false>(&encoded_block::<Standard, false>(&raw));
        assert_ssse3_decode_case::<UrlSafe, false>(&encoded_block::<UrlSafe, false>(&raw));
    }

    for len in [10, 11] {
        fill_pattern(&mut raw, len);
        assert_ssse3_decode_case::<Standard, true>(&encoded_block::<Standard, true>(&raw[..len]));
        assert_ssse3_decode_case::<UrlSafe, true>(&encoded_block::<UrlSafe, true>(&raw[..len]));
    }

    assert_ssse3_decode_error_matches_scalar::<Standard, true>(mutated_block(
        encoded_block::<Standard, true>(&raw),
        5,
        b'!',
    ));
    assert_ssse3_decode_error_matches_scalar::<Standard, true>(mutated_block(
        encoded_block::<Standard, true>(&raw),
        3,
        b'=',
    ));
    assert_ssse3_decode_error_matches_scalar::<UrlSafe, true>(mutated_block(
        encoded_block::<UrlSafe, true>(&raw),
        8,
        b'/',
    ));

    let non_canonical = *b"AAAAAAAAAAAAAB==";
    assert_ssse3_decode_error_matches_scalar::<Standard, true>(non_canonical);
    assert_ssse3_decode_error_matches_scalar::<UrlSafe, true>(non_canonical);
}

#[test]
fn avx2_decode_block_matches_scalar_when_available() {
    if !avx2_available() {
        println!("skipped: AVX2 decode block test requires avx2");
        return;
    }

    let mut raw = [0; 24];
    for seed in 0..64 {
        fill_pattern(&mut raw, seed);
        assert_avx2_decode_case::<Standard, true>(&encoded_wide_block::<Standard, true>(&raw));
        assert_avx2_decode_case::<UrlSafe, true>(&encoded_wide_block::<UrlSafe, true>(&raw));
        assert_avx2_decode_case::<Standard, false>(&encoded_wide_block::<Standard, false>(&raw));
        assert_avx2_decode_case::<UrlSafe, false>(&encoded_wide_block::<UrlSafe, false>(&raw));
    }

    for len in [22, 23] {
        fill_pattern(&mut raw, len);
        assert_avx2_decode_case::<Standard, true>(&encoded_wide_block::<Standard, true>(
            &raw[..len],
        ));
        assert_avx2_decode_case::<UrlSafe, true>(&encoded_wide_block::<UrlSafe, true>(&raw[..len]));
    }

    assert_avx2_decode_error_matches_scalar::<Standard, true>(mutated_wide_block(
        encoded_wide_block::<Standard, true>(&raw),
        17,
        b'!',
    ));
    assert_avx2_decode_error_matches_scalar::<Standard, true>(mutated_wide_block(
        encoded_wide_block::<Standard, true>(&raw),
        11,
        b'=',
    ));
    assert_avx2_decode_error_matches_scalar::<UrlSafe, true>(mutated_wide_block(
        encoded_wide_block::<UrlSafe, true>(&raw),
        12,
        b'/',
    ));

    let mut non_canonical = [b'A'; 32];
    non_canonical[29] = b'B';
    non_canonical[30] = b'=';
    non_canonical[31] = b'=';
    assert_avx2_decode_error_matches_scalar::<Standard, true>(non_canonical);
    assert_avx2_decode_error_matches_scalar::<UrlSafe, true>(non_canonical);
}

fn encoded_block<A, const PAD: bool>(input: &[u8]) -> [u8; 16]
where
    A: Alphabet,
{
    let mut encoded = [0; 16];
    let written = Engine::<A, PAD>::new()
        .encode_slice(input, &mut encoded)
        .unwrap();
    assert_eq!(written, encoded.len());
    encoded
}

fn encoded_wide_block<A, const PAD: bool>(input: &[u8]) -> [u8; 32]
where
    A: Alphabet,
{
    let mut encoded = [0; 32];
    let written = Engine::<A, PAD>::new()
        .encode_slice(input, &mut encoded)
        .unwrap();
    assert_eq!(written, encoded.len());
    encoded
}

fn mutated_block(mut input: [u8; 16], index: usize, byte: u8) -> [u8; 16] {
    input[index] = byte;
    input
}

fn mutated_wide_block(mut input: [u8; 32], index: usize, byte: u8) -> [u8; 32] {
    input[index] = byte;
    input
}

fn assert_ssse3_decode_case<A, const PAD: bool>(input: &[u8; 16])
where
    A: Alphabet,
{
    let mut ssse3 = [0x55; 12];
    let mut scalar = [0xaa; 12];
    // SAFETY: The caller checked SSSE3/SSE4.1 availability before invoking
    // this helper.
    let ssse3_len = unsafe { decode_16_bytes_ssse3_sse41::<A, PAD>(input, &mut ssse3) }.unwrap();
    let scalar_len = Engine::<A, PAD>::new()
        .decode_slice(input, &mut scalar)
        .unwrap();
    assert_eq!(ssse3_len, scalar_len);
    assert_eq!(&ssse3[..ssse3_len], &scalar[..scalar_len]);
}

fn assert_ssse3_decode_error_matches_scalar<A, const PAD: bool>(input: [u8; 16])
where
    A: Alphabet,
{
    let mut ssse3 = [0x55; 12];
    let mut scalar = [0xaa; 12];
    // SAFETY: The caller checked SSSE3/SSE4.1 availability before invoking
    // this helper.
    let ssse3_error = unsafe { decode_16_bytes_ssse3_sse41::<A, PAD>(&input, &mut ssse3) }
        .expect_err("malformed block must be rejected by SSSE3/SSE4.1 prototype");
    let scalar_error = Engine::<A, PAD>::new()
        .decode_slice(&input, &mut scalar)
        .expect_err("malformed block must be rejected by scalar decoder");
    assert_eq!(ssse3_error, scalar_error);
    assert_eq!(ssse3, [0x55; 12]);
    assert!(matches!(
        ssse3_error,
        DecodeError::InvalidByte { .. }
            | DecodeError::InvalidPadding { .. }
            | DecodeError::InvalidLength
    ));
}

fn assert_avx2_decode_case<A, const PAD: bool>(input: &[u8; 32])
where
    A: Alphabet,
{
    let mut avx2 = [0x55; 24];
    let mut scalar = [0xaa; 24];
    // SAFETY: The caller checked AVX2 availability before invoking this
    // helper.
    let avx2_len = unsafe { decode_32_bytes_avx2::<A, PAD>(input, &mut avx2) }.unwrap();
    let scalar_len = Engine::<A, PAD>::new()
        .decode_slice(input, &mut scalar)
        .unwrap();
    assert_eq!(avx2_len, scalar_len);
    assert_eq!(&avx2[..avx2_len], &scalar[..scalar_len]);
}

fn assert_avx2_decode_error_matches_scalar<A, const PAD: bool>(input: [u8; 32])
where
    A: Alphabet,
{
    let mut avx2 = [0x55; 24];
    let mut scalar = [0xaa; 24];
    // SAFETY: The caller checked AVX2 availability before invoking this
    // helper.
    let avx2_error = unsafe { decode_32_bytes_avx2::<A, PAD>(&input, &mut avx2) }
        .expect_err("malformed block must be rejected by AVX2 prototype");
    let scalar_error = Engine::<A, PAD>::new()
        .decode_slice(&input, &mut scalar)
        .expect_err("malformed block must be rejected by scalar decoder");
    assert_eq!(avx2_error, scalar_error);
    assert_eq!(avx2, [0x55; 24]);
    assert!(matches!(
        avx2_error,
        DecodeError::InvalidByte { .. }
            | DecodeError::InvalidPadding { .. }
            | DecodeError::InvalidLength
    ));
}
