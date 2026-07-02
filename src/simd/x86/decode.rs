#![allow(unsafe_code)]

use crate::{Alphabet, DecodeError, scalar};

#[cfg(target_arch = "x86")]
use core::arch::x86::{
    __m128i, __m256i, __m512i, _mm_loadu_si128, _mm_madd_epi16, _mm_maddubs_epi16, _mm_set1_epi32,
    _mm_setr_epi8, _mm_shuffle_epi8, _mm_storeu_si128, _mm256_loadu_si256, _mm256_madd_epi16,
    _mm256_maddubs_epi16, _mm256_set1_epi32, _mm256_setr_epi8, _mm256_shuffle_epi8,
    _mm256_storeu_si256, _mm512_loadu_si512, _mm512_madd_epi16, _mm512_maddubs_epi16,
    _mm512_permutexvar_epi8, _mm512_set1_epi32, _mm512_shuffle_epi8, _mm512_storeu_si512,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m128i, __m256i, __m512i, _mm_loadu_si128, _mm_madd_epi16, _mm_maddubs_epi16, _mm_set1_epi32,
    _mm_setr_epi8, _mm_shuffle_epi8, _mm_storeu_si128, _mm256_loadu_si256, _mm256_madd_epi16,
    _mm256_maddubs_epi16, _mm256_set1_epi32, _mm256_setr_epi8, _mm256_shuffle_epi8,
    _mm256_storeu_si256, _mm512_loadu_si512, _mm512_madd_epi16, _mm512_maddubs_epi16,
    _mm512_permutexvar_epi8, _mm512_set1_epi32, _mm512_shuffle_epi8, _mm512_storeu_si512,
};

#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm_loadu_si128 and _mm_storeu_si128 accept unaligned pointers"
)]
#[target_feature(enable = "ssse3,sse4.1")]
pub(crate) unsafe fn decode_16_bytes_ssse3_sse41<A, const PAD: bool>(
    input: &[u8; 16],
    output: &mut [u8; 12],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    let mut scalar_output = [0; 12];
    let written = match scalar::decode_slice::<A, PAD>(input, &mut scalar_output) {
        Ok(written) => written,
        Err(error) => {
            crate::wipe_bytes(&mut scalar_output);
            return Err(error);
        }
    };

    if !super::is_standard_or_url_safe_family::<A>() {
        output[..written].copy_from_slice(&scalar_output[..written]);
        crate::wipe_bytes(&mut scalar_output);
        return Ok(written);
    }

    let mut values = [0; 16];
    fill_decode_values::<A, 16>(input, &mut values);
    let mut packed = [0; 16];

    // SAFETY: Fixed arrays back the unaligned loads and stores, the
    // target-feature contract enables SSSE3/SSE4.1, and scalar validation
    // above proves the 16-byte block is canonical for the selected padding
    // policy before any bytes are copied to the caller output.
    unsafe {
        let values_vec = _mm_loadu_si128(values.as_ptr().cast::<__m128i>());
        let merged_pairs = _mm_maddubs_epi16(values_vec, _mm_set1_epi32(0x0140_0140));
        let merged_quads = _mm_madd_epi16(merged_pairs, _mm_set1_epi32(0x0001_1000));
        let shuffle = _mm_setr_epi8(
            2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -128, -128, -128, -128,
        );
        let decoded = _mm_shuffle_epi8(merged_quads, shuffle);
        _mm_storeu_si128(packed.as_mut_ptr().cast::<__m128i>(), decoded);
        super::cleanup::clear_xmm_registers_after_encode_block();
    }

    debug_assert_eq!(&packed[..written], &scalar_output[..written]);
    output[..written].copy_from_slice(&packed[..written]);
    crate::wipe_bytes(&mut values);
    crate::wipe_bytes(&mut packed);
    crate::wipe_bytes(&mut scalar_output);
    Ok(written)
}

#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm256_loadu_si256 and _mm256_storeu_si256 accept unaligned pointers"
)]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn decode_32_bytes_avx2<A, const PAD: bool>(
    input: &[u8; 32],
    output: &mut [u8; 24],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    let mut scalar_output = [0; 24];
    let written = match scalar::decode_slice::<A, PAD>(input, &mut scalar_output) {
        Ok(written) => written,
        Err(error) => {
            crate::wipe_bytes(&mut scalar_output);
            return Err(error);
        }
    };

    if !super::is_standard_or_url_safe_family::<A>() {
        output[..written].copy_from_slice(&scalar_output[..written]);
        crate::wipe_bytes(&mut scalar_output);
        return Ok(written);
    }

    let mut values = [0; 32];
    fill_decode_values::<A, 32>(input, &mut values);
    let mut packed = [0; 32];

    // SAFETY: Fixed arrays back the unaligned loads and stores, the
    // target-feature contract enables AVX2, and scalar validation above
    // proves the 32-byte block is canonical for the selected padding policy
    // before any bytes are copied to the caller output.
    unsafe {
        let values_vec = _mm256_loadu_si256(values.as_ptr().cast::<__m256i>());
        let merged_pairs = _mm256_maddubs_epi16(values_vec, _mm256_set1_epi32(0x0140_0140));
        let merged_quads = _mm256_madd_epi16(merged_pairs, _mm256_set1_epi32(0x0001_1000));
        let shuffle = _mm256_setr_epi8(
            2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -128, -128, -128, -128, 2, 1, 0, 6, 5, 4, 10,
            9, 8, 14, 13, 12, -128, -128, -128, -128,
        );
        let decoded = _mm256_shuffle_epi8(merged_quads, shuffle);
        _mm256_storeu_si256(packed.as_mut_ptr().cast::<__m256i>(), decoded);
        super::cleanup::clear_ymm_registers_after_encode_block();
    }
    let upper_lane = [
        packed[16], packed[17], packed[18], packed[19], packed[20], packed[21], packed[22],
        packed[23], packed[24], packed[25], packed[26], packed[27],
    ];
    packed[12..24].copy_from_slice(&upper_lane);

    debug_assert_eq!(&packed[..written], &scalar_output[..written]);
    output[..written].copy_from_slice(&packed[..written]);
    crate::wipe_bytes(&mut values);
    crate::wipe_bytes(&mut packed);
    crate::wipe_bytes(&mut scalar_output);
    Ok(written)
}

#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm512_loadu_si512 and _mm512_storeu_si512 accept unaligned pointers"
)]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
pub(crate) unsafe fn decode_64_bytes_avx512<A, const PAD: bool>(
    input: &[u8; 64],
    output: &mut [u8; 48],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    let mut scalar_output = [0; 48];
    let written = match scalar::decode_slice::<A, PAD>(input, &mut scalar_output) {
        Ok(written) => written,
        Err(error) => {
            crate::wipe_bytes(&mut scalar_output);
            return Err(error);
        }
    };

    if !super::is_standard_or_url_safe_family::<A>() {
        output[..written].copy_from_slice(&scalar_output[..written]);
        crate::wipe_bytes(&mut scalar_output);
        return Ok(written);
    }

    let mut values = [0; 64];
    fill_decode_values::<A, 64>(input, &mut values);
    let mut packed = [0; 64];
    let shuffle_mask: [i8; 64] = [
        2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -128, -128, -128, -128, 2, 1, 0, 6, 5, 4, 10, 9, 8,
        14, 13, 12, -128, -128, -128, -128, 2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -128, -128,
        -128, -128, 2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -128, -128, -128, -128,
    ];
    let compact_indices: [u8; 64] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 32,
        33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    // SAFETY: Fixed arrays back every unaligned load and store, scalar
    // validation above proves the 64-byte block is canonical for the selected
    // padding policy, and the AVX-512/VBMI target-feature contract enables
    // the packing, lane compaction, and cleanup instructions used here.
    unsafe {
        let values_vec = _mm512_loadu_si512(values.as_ptr().cast::<__m512i>());
        let merged_pairs = _mm512_maddubs_epi16(values_vec, _mm512_set1_epi32(0x0140_0140));
        let merged_quads = _mm512_madd_epi16(merged_pairs, _mm512_set1_epi32(0x0001_1000));
        let shuffle = _mm512_loadu_si512(shuffle_mask.as_ptr().cast::<__m512i>());
        let decoded_lanes = _mm512_shuffle_epi8(merged_quads, shuffle);
        let compact = _mm512_loadu_si512(compact_indices.as_ptr().cast::<__m512i>());
        let decoded = _mm512_permutexvar_epi8(compact, decoded_lanes);
        _mm512_storeu_si512(packed.as_mut_ptr().cast::<__m512i>(), decoded);
        super::cleanup::clear_zmm_registers_after_encode_block();
    }

    debug_assert_eq!(&packed[..written], &scalar_output[..written]);
    output[..written].copy_from_slice(&packed[..written]);
    crate::wipe_bytes(&mut values);
    crate::wipe_bytes(&mut packed);
    crate::wipe_bytes(&mut scalar_output);
    Ok(written)
}

fn fill_decode_values<A, const N: usize>(input: &[u8; N], values: &mut [u8; N])
where
    A: Alphabet,
{
    let mut index = 0;
    while index < input.len() {
        values[index] = if input[index] == b'=' {
            0
        } else {
            A::decode(input[index]).unwrap_or(0)
        };
        index += 1;
    }
}
