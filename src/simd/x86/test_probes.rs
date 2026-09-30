use crate::Alphabet;

pub(crate) fn candidate_validate_avx512(input: &[u8], url_safe: bool) -> bool {
    if !crate::simd::avx512_vbmi_base64_available() || !input.len().is_multiple_of(64) {
        return false;
    }
    let blocks = input.as_chunks::<64>().0;
    // SAFETY: The full feature/OS-state probe and exact arrays bound all loads.
    // Only the closed Standard/URL-safe families reach this classifier.
    let valid = unsafe {
        if url_safe {
            super::decode_direct::validate_blocks_avx512::<crate::UrlSafe>(blocks)
        } else {
            super::decode_direct::validate_blocks_avx512::<crate::Standard>(blocks)
        }
    };
    // SAFETY: All vector/mask results are dead, including on early rejection.
    unsafe { super::cleanup::clear_zmm_registers_after_encode_block() };
    valid
}

pub(crate) fn candidate_decode_avx512(input: &[u8], output: &mut [u8], url_safe: bool) -> bool {
    let required = input.len() / 64 * 48;
    if !crate::simd::avx512_vbmi_base64_available()
        || !input.len().is_multiple_of(64)
        || output.len() < required
    {
        return false;
    }
    // SAFETY: Full CPU/OS support and 64:48 geometry were checked. The existing
    // loop validates each block before its exact 48-byte masked store.
    let (read, written, valid) = unsafe {
        if url_safe {
            super::decode::decode_full_blocks_avx512::<crate::UrlSafe>(input, output, input.len())
        } else {
            super::decode::decode_full_blocks_avx512::<crate::Standard>(input, output, input.len())
        }
    };
    valid && read == input.len() && written == required
}

pub(crate) fn test_avx512_loop_geometry(
    input: &[u8],
    output: &mut [u8],
    requested: usize,
    url_safe: bool,
) -> Option<(usize, usize, bool)> {
    if !crate::simd::avx512_vbmi_base64_available() {
        return None;
    }
    // SAFETY: CPU/OS support is the only unsafe precondition. Deliberately do
    // not precheck geometry: tests must exercise the loop's own bounds checks.
    Some(unsafe {
        if url_safe {
            super::decode::decode_full_blocks_avx512::<crate::UrlSafe>(input, output, requested)
        } else {
            super::decode::decode_full_blocks_avx512::<crate::Standard>(input, output, requested)
        }
    })
}

pub(crate) fn candidate_validate_avx2(input: &[u8], url_safe: bool) -> bool {
    if !crate::simd::avx2_available() || !input.len().is_multiple_of(32) {
        return false;
    }
    let blocks = input.as_chunks::<32>().0;
    // SAFETY: The probe proves AVX2, arrays bound every load, and the
    // closed choice excludes custom alphabets. The classifier has no stores.
    let valid = unsafe {
        if url_safe {
            super::decode_direct::validate_blocks_avx2::<crate::UrlSafe>(blocks)
        } else {
            super::decode_direct::validate_blocks_avx2::<crate::Standard>(blocks)
        }
    };
    // SAFETY: Classification is complete, including any early rejection.
    unsafe { super::cleanup::clear_ymm_registers_after_encode_block() };
    valid
}

pub(crate) fn candidate_decode_avx2(input: &[u8], output: &mut [u8], url_safe: bool) -> bool {
    let required = input.len() / 32 * 24;
    if !crate::simd::avx2_available() || !input.len().is_multiple_of(32) || output.len() < required
    {
        return false;
    }
    // SAFETY: CPU availability and exact 32:24 geometry are established above.
    // The reviewed kernel checks alphabet validity before each block store.
    let (read, written, valid) = unsafe {
        if url_safe {
            super::decode::decode_full_blocks_avx2::<crate::UrlSafe>(input, output, input.len())
        } else {
            super::decode::decode_full_blocks_avx2::<crate::Standard>(input, output, input.len())
        }
    };
    // SAFETY: Also clear the first-block rejection case, when the existing
    // loop's stored-block cleanup has not run. No vector results remain live.
    unsafe { super::cleanup::clear_ymm_registers_after_encode_block() };
    valid && read == input.len() && written == required
}

pub(crate) fn candidate_validate_16(input: &[u8; 16], url_safe: bool) -> bool {
    if !crate::simd::ssse3_sse41_available() {
        return false;
    }
    // SAFETY: The runtime probe proves the feature bundle, arrays bound loads,
    // and these closed choices cannot introduce a custom alphabet.
    let valid = unsafe {
        if url_safe {
            super::decode_direct::validate_16_bytes_ssse3_sse41::<crate::UrlSafe>(input)
        } else {
            super::decode_direct::validate_16_bytes_ssse3_sse41::<crate::Standard>(input)
        }
    };
    // SAFETY: The classification result is scalar; vector temporaries are dead.
    unsafe { super::cleanup::clear_xmm_registers_after_encode_block() };
    valid
}

pub(crate) fn candidate_decode_16(input: &[u8; 16], output: &mut [u8; 12], url_safe: bool) -> bool {
    if url_safe {
        test_direct_decode_16::<crate::UrlSafe>(input, output)
    } else {
        test_direct_decode_16::<crate::Standard>(input, output)
    }
}

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
