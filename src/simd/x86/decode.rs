#![allow(unsafe_code)]

use crate::{Alphabet, DecodeError, scalar};

use super::super::decode_helpers::{copy_verified_decode_output, fill_decode_values};

const SSSE3_DECODE_INPUT_BLOCK: usize = 16;
const SSSE3_DECODE_OUTPUT_BLOCK: usize = 12;
const AVX2_DECODE_INPUT_BLOCK: usize = 32;
const AVX2_DECODE_OUTPUT_BLOCK: usize = 24;
const AVX512_DECODE_INPUT_BLOCK: usize = 64;
const AVX512_DECODE_OUTPUT_BLOCK: usize = 48;

pub(crate) fn decode_slice_ssse3_sse41<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    if input.len() < SSSE3_DECODE_INPUT_BLOCK || !super::ssse3_sse41_supports_alphabet::<A>() {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    let required = scalar::validate_decode::<A, PAD>(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let simd_input_len = unpadded_simd_prefix_len(input);
    // SAFETY: Health-gated dispatch or the static token contract proves the
    // target features. Validation and the output preflight above prove every
    // full block is canonical and every direct store remains in bounds.
    let (read, write, classified) =
        unsafe { decode_full_blocks_ssse3_sse41::<A>(input, output, simd_input_len) };
    if !classified {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    let tail_written = scalar::decode_slice::<A, PAD>(&input[read..], &mut output[write..])
        .map_err(|error| error.with_index_offset(read))?;
    Ok(write + tail_written)
}

pub(crate) fn decode_slice_avx2<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    if input.len() < AVX2_DECODE_INPUT_BLOCK || !super::avx2_supports_alphabet::<A>() {
        return decode_slice_ssse3_sse41::<A, PAD>(input, output);
    }

    let required = scalar::validate_decode::<A, PAD>(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let simd_input_len = unpadded_simd_prefix_len(input);
    // SAFETY: Health-gated dispatch or the static token contract proves AVX2.
    // Validation and the output preflight above prove exact block bounds.
    let (read, write, classified) =
        unsafe { decode_full_blocks_avx2::<A>(input, output, simd_input_len) };
    if !classified {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    let tail_written = decode_slice_ssse3_sse41::<A, PAD>(&input[read..], &mut output[write..])
        .map_err(|error| error.with_index_offset(read))?;
    Ok(write + tail_written)
}

fn unpadded_simd_prefix_len(input: &[u8]) -> usize {
    if input.last() == Some(&b'=') {
        input.len().saturating_sub(4)
    } else {
        input.len()
    }
}

pub(crate) fn decode_slice_avx512<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    if input.len() < AVX512_DECODE_INPUT_BLOCK || !super::avx512_supports_alphabet::<A>() {
        return decode_slice_avx2::<A, PAD>(input, output);
    }

    let required = scalar::validate_decode::<A, PAD>(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + AVX512_DECODE_INPUT_BLOCK <= input.len() {
        let mut decoded = [0u8; AVX512_DECODE_OUTPUT_BLOCK];
        // SAFETY: Health-gated dispatch reaches this function only after
        // runtime probing, complete static target features, or the unsafe
        // token contract proves AVX-512 VBMI availability. The loop guard
        // proves the fixed input view is in bounds. Whole-input scalar
        // validation above preserves public error shape before any bytes are
        // copied to caller output.
        let written = match unsafe {
            let block = &*(input
                .as_ptr()
                .add(read)
                .cast::<[u8; AVX512_DECODE_INPUT_BLOCK]>());
            decode_64_bytes_avx512::<A, PAD>(block, &mut decoded)
        } {
            Ok(written) => written,
            Err(error) => {
                crate::wipe_bytes(&mut decoded);
                return Err(error.with_index_offset(read));
            }
        };

        output[write..write + written].copy_from_slice(&decoded[..written]);
        crate::wipe_bytes(&mut decoded);
        read += AVX512_DECODE_INPUT_BLOCK;
        write += written;
    }

    let tail_written = decode_slice_avx2::<A, PAD>(&input[read..], &mut output[write..])
        .map_err(|error| error.with_index_offset(read))?;
    Ok(write + tail_written)
}

#[cfg(target_arch = "x86")]
use core::arch::x86::{
    __m512i, _mm512_loadu_si512, _mm512_madd_epi16, _mm512_maddubs_epi16, _mm512_permutexvar_epi8,
    _mm512_set1_epi32, _mm512_shuffle_epi8, _mm512_storeu_si512,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m512i, _mm512_loadu_si512, _mm512_madd_epi16, _mm512_maddubs_epi16, _mm512_permutexvar_epi8,
    _mm512_set1_epi32, _mm512_shuffle_epi8, _mm512_storeu_si512,
};

#[cfg(all(feature = "std", test))]
#[target_feature(enable = "ssse3,sse4.1")]
pub(crate) unsafe fn decode_16_bytes_ssse3_sse41<A, const PAD: bool>(
    input: &[u8; 16],
    output: &mut [u8; 12],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    let written = scalar::validate_decode::<A, PAD>(input)?;
    if written != SSSE3_DECODE_OUTPUT_BLOCK
        || !super::is_standard_or_url_safe_family::<A>()
        || input.contains(&b'=')
    {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    // SAFETY: This function carries the target-feature contract. Scalar
    // validation proves the unpadded block is canonical, and fixed arrays
    // prove the exact 16-byte load and 12-byte store bounds.
    let classified =
        unsafe { super::decode_direct::decode_16_bytes_ssse3_sse41::<A>(input, output) };
    // SAFETY: The direct block no longer needs SIMD register contents.
    unsafe { super::cleanup::clear_xmm_registers_after_encode_block() };
    if !classified {
        return scalar::decode_slice::<A, PAD>(input, output);
    }
    Ok(SSSE3_DECODE_OUTPUT_BLOCK)
}

#[cfg(all(feature = "std", test))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn decode_32_bytes_avx2<A, const PAD: bool>(
    input: &[u8; 32],
    output: &mut [u8; 24],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    let written = scalar::validate_decode::<A, PAD>(input)?;
    if written != AVX2_DECODE_OUTPUT_BLOCK
        || !super::is_standard_or_url_safe_family::<A>()
        || input.contains(&b'=')
    {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    // SAFETY: This function carries the target-feature contract. Scalar
    // validation proves the unpadded block is canonical, and fixed arrays
    // prove the exact 32-byte load and two 12-byte stores remain in bounds.
    let classified = unsafe { super::decode_direct::decode_32_bytes_avx2::<A>(input, output) };
    // SAFETY: The direct block no longer needs SIMD register contents.
    unsafe { super::cleanup::clear_ymm_registers_after_encode_block() };
    if !classified {
        return scalar::decode_slice::<A, PAD>(input, output);
    }
    Ok(AVX2_DECODE_OUTPUT_BLOCK)
}

#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn decode_full_blocks_ssse3_sse41<A>(
    input: &[u8],
    output: &mut [u8],
    simd_input_len: usize,
) -> (usize, usize, bool)
where
    A: Alphabet,
{
    let mut read = 0;
    let mut write = 0;
    let mut classified = true;
    while read + SSSE3_DECODE_INPUT_BLOCK <= simd_input_len {
        // SAFETY: The loop guards prove both exact fixed blocks are within the
        // preflighted slices. This function carries the ISA contract.
        let block_classified = unsafe {
            let block = &*(input
                .as_ptr()
                .add(read)
                .cast::<[u8; SSSE3_DECODE_INPUT_BLOCK]>());
            let decoded = &mut *(output
                .as_mut_ptr()
                .add(write)
                .cast::<[u8; SSSE3_DECODE_OUTPUT_BLOCK]>());
            super::decode_direct::decode_16_bytes_ssse3_sse41::<A>(block, decoded)
        };
        if !block_classified {
            classified = false;
            break;
        }
        read += SSSE3_DECODE_INPUT_BLOCK;
        write += SSSE3_DECODE_OUTPUT_BLOCK;
    }
    if read != 0 {
        // SAFETY: All vector results have already been stored.
        unsafe { super::cleanup::clear_xmm_registers_after_encode_block() };
    }
    (read, write, classified)
}

#[target_feature(enable = "avx2")]
unsafe fn decode_full_blocks_avx2<A>(
    input: &[u8],
    output: &mut [u8],
    simd_input_len: usize,
) -> (usize, usize, bool)
where
    A: Alphabet,
{
    let mut read = 0;
    let mut write = 0;
    let mut classified = true;
    while read + AVX2_DECODE_INPUT_BLOCK <= simd_input_len {
        // SAFETY: The loop guards prove both exact fixed blocks are within the
        // preflighted slices. This function carries the ISA contract.
        let block_classified = unsafe {
            let block = &*(input
                .as_ptr()
                .add(read)
                .cast::<[u8; AVX2_DECODE_INPUT_BLOCK]>());
            let decoded = &mut *(output
                .as_mut_ptr()
                .add(write)
                .cast::<[u8; AVX2_DECODE_OUTPUT_BLOCK]>());
            super::decode_direct::decode_32_bytes_avx2::<A>(block, decoded)
        };
        if !block_classified {
            classified = false;
            break;
        }
        read += AVX2_DECODE_INPUT_BLOCK;
        write += AVX2_DECODE_OUTPUT_BLOCK;
    }
    if read != 0 {
        // SAFETY: All vector results have already been stored.
        unsafe { super::cleanup::clear_ymm_registers_after_encode_block() };
    }
    (read, write, classified)
}

#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm512_loadu_si512 and _mm512_storeu_si512 accept unaligned pointers"
)]
#[cfg_attr(not(test), allow(dead_code))]
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

    crate::wipe_bytes(&mut values);
    copy_verified_decode_output(&mut packed, &mut scalar_output, output, written)?;
    Ok(written)
}
