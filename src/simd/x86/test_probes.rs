use crate::Alphabet;

pub(super) fn scalar_encode_block<A, const IN: usize, const OUT: usize>(
    input: &[u8; IN],
    output: &mut [u8; OUT],
) where
    A: Alphabet,
{
    let mut read = 0;
    let mut write = 0;
    while read < input.len() {
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];

        output[write] = crate::encode_base64_value::<A>(b0 >> 2);
        output[write + 1] = crate::encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
        output[write + 2] = crate::encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
        output[write + 3] = crate::encode_base64_value::<A>(b2 & 0b0011_1111);

        read += 3;
        write += 4;
    }
}

pub(in crate::simd) fn test_direct_decode_16<A: Alphabet>(
    input: &[u8; 16],
    output: &mut [u8; 12],
) -> bool {
    if !crate::simd::ssse3_sse41_available() {
        return false;
    }
    // SAFETY: The runtime probe proves the complete feature contract; fixed
    // arrays prove exact block bounds.
    let classified =
        unsafe { super::decode_direct::decode_16_bytes_ssse3_sse41::<A>(input, output) };
    // SAFETY: The direct block has stored or rejected all vector output.
    unsafe { super::cleanup::clear_xmm_registers_after_encode_block() };
    classified
}

pub(in crate::simd) fn test_direct_decode_32<A: Alphabet>(
    input: &[u8; 32],
    output: &mut [u8; 24],
) -> bool {
    if !crate::simd::avx2_available() {
        return false;
    }
    // SAFETY: The runtime probe proves AVX2; fixed arrays prove exact block
    // bounds.
    let classified = unsafe { super::decode_direct::decode_32_bytes_avx2::<A>(input, output) };
    // SAFETY: The direct block has stored or rejected all vector output.
    unsafe { super::cleanup::clear_ymm_registers_after_encode_block() };
    classified
}

pub(in crate::simd) fn test_direct_decode_64<A: Alphabet>(
    input: &[u8; 64],
    output: &mut [u8; 48],
) -> bool {
    if !crate::simd::avx512_vbmi_base64_available() {
        return false;
    }
    // SAFETY: The runtime probe proves the complete AVX-512 VBMI feature
    // contract; fixed arrays prove exact block bounds.
    let classified = unsafe { super::decode_direct::decode_64_bytes_avx512::<A>(input, output) };
    // SAFETY: The direct block has stored or rejected all vector output.
    unsafe { super::cleanup::clear_zmm_registers_after_encode_block() };
    classified
}
