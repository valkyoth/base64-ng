use crate::{Engine, Standard, UrlSafe};

use super::{decode_16_bytes_neon, neon_available};

#[test]
fn neon_decode_block_matches_scalar_when_available() {
    if !neon_available() {
        println!("skipped: NEON decode block test requires aarch64 NEON");
        return;
    }

    for seed in 0..64 {
        let input = encoded_neon_block::<Standard, true>(seed, 12);
        assert_neon_decode_case::<Standard, true>(&input);

        let input = encoded_neon_block::<UrlSafe, true>(seed, 12);
        assert_neon_decode_case::<UrlSafe, true>(&input);
    }

    let padded_one = encoded_neon_block::<Standard, true>(7, 11);
    assert_neon_decode_case::<Standard, true>(&padded_one);

    let padded_two = encoded_neon_block::<Standard, true>(11, 10);
    assert_neon_decode_case::<Standard, true>(&padded_two);

    let unpadded = encoded_neon_block::<UrlSafe, false>(19, 12);
    assert_neon_decode_case::<UrlSafe, false>(&unpadded);

    let invalid_byte = mutated_neon_block::<Standard, true>(23, 12, 5, b'!');
    assert_neon_decode_error_matches_scalar::<Standard, true>(&invalid_byte);

    let misplaced_padding = mutated_neon_block::<Standard, true>(29, 12, 4, b'=');
    assert_neon_decode_error_matches_scalar::<Standard, true>(&misplaced_padding);

    let mixed_alphabet = mutated_neon_block::<UrlSafe, true>(31, 12, 8, b'/');
    assert_neon_decode_error_matches_scalar::<UrlSafe, true>(&mixed_alphabet);

    let mut non_canonical = [b'A'; 16];
    non_canonical[13] = b'B';
    non_canonical[14] = b'=';
    non_canonical[15] = b'=';
    assert_neon_decode_error_matches_scalar::<Standard, true>(&non_canonical);
}

fn encoded_neon_block<A, const PAD: bool>(seed: usize, raw_len: usize) -> [u8; 16]
where
    A: crate::Alphabet,
{
    let mut raw = [0; 12];
    for (index, byte) in raw.iter_mut().enumerate() {
        let value = (index * 41 + seed * 13) % 256;
        *byte = u8::try_from(value).expect("pattern byte fits in u8");
    }

    let mut encoded = [0; 16];
    let written = Engine::<A, PAD>::new()
        .encode_slice(&raw[..raw_len], &mut encoded)
        .expect("test input must encode");
    assert_eq!(written, 16);
    encoded
}

fn mutated_neon_block<A, const PAD: bool>(
    seed: usize,
    raw_len: usize,
    index: usize,
    byte: u8,
) -> [u8; 16]
where
    A: crate::Alphabet,
{
    let mut input = encoded_neon_block::<A, PAD>(seed, raw_len);
    input[index] = byte;
    input
}

fn assert_neon_decode_case<A, const PAD: bool>(input: &[u8; 16])
where
    A: crate::Alphabet,
{
    let mut neon_output = [0x55; 12];
    let mut scalar_output = [0xaa; 12];

    // SAFETY: The test checked NEON availability before invoking this helper.
    let neon_written = unsafe { decode_16_bytes_neon::<A, PAD>(input, &mut neon_output) }
        .expect("canonical block must decode through NEON prototype");
    let scalar_written = Engine::<A, PAD>::new()
        .decode_slice(input, &mut scalar_output)
        .expect("canonical block must decode through scalar");

    assert_eq!(neon_written, scalar_written);
    assert_eq!(
        &neon_output[..neon_written],
        &scalar_output[..scalar_written]
    );
}

fn assert_neon_decode_error_matches_scalar<A, const PAD: bool>(input: &[u8; 16])
where
    A: crate::Alphabet,
{
    let mut neon_output = [0x55; 12];
    let mut scalar_output = [0xaa; 12];

    // SAFETY: The test checked NEON availability before invoking this helper.
    let neon_error = unsafe { decode_16_bytes_neon::<A, PAD>(input, &mut neon_output) }
        .expect_err("malformed block must be rejected by NEON prototype");
    let scalar_error = Engine::<A, PAD>::new()
        .decode_slice(input, &mut scalar_output)
        .expect_err("malformed block must be rejected by scalar");

    assert_eq!(neon_error, scalar_error);
    assert_eq!(neon_output, [0x55; 12]);
}
