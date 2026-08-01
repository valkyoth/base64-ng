#![allow(unsafe_code)]

mod cleanup;
mod decode;

use crate::{Alphabet, EncodeError, checked_encoded_len, encode_base64_value, scalar};

use cleanup::clear_zmm_registers_after_encode_block;
pub(crate) use decode::decode_slice_avx2;
pub(crate) use decode::decode_slice_avx512;
pub(crate) use decode::decode_slice_ssse3_sse41;
#[cfg(all(feature = "std", test))]
pub(crate) use decode::{
    decode_16_bytes_ssse3_sse41, decode_32_bytes_avx2, decode_64_bytes_avx512,
};

pub(crate) fn avx512_supports_alphabet<A>() -> bool
where
    A: Alphabet,
{
    is_standard_or_url_safe_family::<A>()
}

pub(crate) fn avx2_supports_alphabet<A>() -> bool
where
    A: Alphabet,
{
    is_standard_or_url_safe_family::<A>()
}

pub(crate) fn ssse3_sse41_supports_alphabet<A>() -> bool
where
    A: Alphabet,
{
    is_standard_or_url_safe_family::<A>()
}

pub(crate) fn ssse3_sse41_decode_available() -> bool {
    super::ssse3_sse41_available()
}

pub(crate) fn avx2_decode_available() -> bool {
    super::avx2_available()
}

pub(crate) fn avx512_decode_available() -> bool {
    super::avx512_vbmi_base64_available()
}

pub(crate) fn encode_slice_avx512<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    if input.len() < 48 {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    if !avx512_supports_alphabet::<A>() {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
    if output.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + 48 <= input.len() {
        // SAFETY: Health admission proves AVX-512 VBMI support. The loop and
        // prevalidated output length prove both fixed-size views are in bounds.
        unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; 48]>());
            let encoded = &mut *(output.as_mut_ptr().add(write).cast::<[u8; 64]>());
            encode_48_bytes_avx512::<A>(block, encoded);
        }
        read += 48;
        write += 64;
    }

    let tail_written = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail_written)
}

pub(crate) fn encode_slice_avx2<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    if input.len() < 24 {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    if !avx2_supports_alphabet::<A>() {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
    if output.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    // SAFETY: Health admission proves AVX2 support. The output preflight and
    // fixed block ratio prove every direct load and store remains in bounds.
    let (read, write) = unsafe { encode_full_blocks_avx2::<A>(input, output) };

    let tail_written = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail_written)
}

pub(crate) fn encode_slice_ssse3_sse41<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    if input.len() < 12 {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    if !ssse3_sse41_supports_alphabet::<A>() {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
    if output.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    // SAFETY: Health admission proves SSSE3/SSE4.1 support. The output
    // preflight and fixed block ratio prove every direct access is in bounds.
    let (read, write) = unsafe { encode_full_blocks_ssse3_sse41::<A>(input, output) };

    let tail_written = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail_written)
}

#[cfg(target_arch = "x86")]
use core::arch::x86::{
    __m128i, __m256i, __m512i, _mm_add_epi8, _mm_and_si128, _mm_cmpgt_epi8, _mm_cvtsi32_si128,
    _mm_loadl_epi64, _mm_loadu_si128, _mm_or_si128, _mm_set_epi32, _mm_set1_epi8, _mm_set1_epi32,
    _mm_setr_epi8, _mm_shuffle_epi8, _mm_slli_epi32, _mm_slli_si128, _mm_srli_epi32,
    _mm_srli_si128, _mm_storeu_si128, _mm_sub_epi8, _mm_subs_epu8, _mm256_add_epi8,
    _mm256_and_si256, _mm256_broadcastsi128_si256, _mm256_castsi128_si256, _mm256_cmpgt_epi8,
    _mm256_inserti128_si256, _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32,
    _mm256_setr_epi8, _mm256_shuffle_epi8, _mm256_slli_epi32, _mm256_srli_epi32,
    _mm256_storeu_si256, _mm256_sub_epi8, _mm256_subs_epu8, _mm512_and_si512, _mm512_loadu_si512,
    _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi32, _mm512_shuffle_epi8,
    _mm512_slli_epi32, _mm512_srli_epi32, _mm512_storeu_si512,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m128i, __m256i, __m512i, _mm_add_epi8, _mm_and_si128, _mm_cmpgt_epi8, _mm_cvtsi32_si128,
    _mm_loadl_epi64, _mm_loadu_si128, _mm_or_si128, _mm_set_epi32, _mm_set1_epi8, _mm_set1_epi32,
    _mm_setr_epi8, _mm_shuffle_epi8, _mm_slli_epi32, _mm_slli_si128, _mm_srli_epi32,
    _mm_srli_si128, _mm_storeu_si128, _mm_sub_epi8, _mm_subs_epu8, _mm256_add_epi8,
    _mm256_and_si256, _mm256_broadcastsi128_si256, _mm256_castsi128_si256, _mm256_cmpgt_epi8,
    _mm256_inserti128_si256, _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32,
    _mm256_setr_epi8, _mm256_shuffle_epi8, _mm256_slli_epi32, _mm256_srli_epi32,
    _mm256_storeu_si256, _mm256_sub_epi8, _mm256_subs_epu8, _mm512_and_si512, _mm512_loadu_si512,
    _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi32, _mm512_shuffle_epi8,
    _mm512_slli_epi32, _mm512_srli_epi32, _mm512_storeu_si512,
};

#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm512_storeu_si512 accepts unaligned pointers"
)]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
pub(crate) unsafe fn encode_48_bytes_avx512<A>(input: &[u8; 48], output: &mut [u8; 64])
where
    A: Alphabet,
{
    if !is_standard_or_url_safe_family::<A>() {
        scalar_encode_block::<A, 48, 64>(input, output);
        return;
    }

    let mut staged = [
        input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        input[9], input[10], input[11], 0, 0, 0, 0, input[12], input[13], input[14], input[15],
        input[16], input[17], input[18], input[19], input[20], input[21], input[22], input[23], 0,
        0, 0, 0, input[24], input[25], input[26], input[27], input[28], input[29], input[30],
        input[31], input[32], input[33], input[34], input[35], 0, 0, 0, 0, input[36], input[37],
        input[38], input[39], input[40], input[41], input[42], input[43], input[44], input[45],
        input[46], input[47], 0, 0, 0, 0,
    ];
    let table = A::ENCODE;
    let shuffle_mask: [i8; 64] = [
        2, 1, 0, -128, 5, 4, 3, -128, 8, 7, 6, -128, 11, 10, 9, -128, 2, 1, 0, -128, 5, 4, 3, -128,
        8, 7, 6, -128, 11, 10, 9, -128, 2, 1, 0, -128, 5, 4, 3, -128, 8, 7, 6, -128, 11, 10, 9,
        -128, 2, 1, 0, -128, 5, 4, 3, -128, 8, 7, 6, -128, 11, 10, 9, -128,
    ];

    // SAFETY: Fixed arrays back every unaligned 512-bit load/store, the
    // target-feature contract enables AVX-512/VBMI, shuffle zero lanes read
    // only staged zeros, and VBMI indices are masked to `0..=63`.
    unsafe {
        let input_vec = _mm512_loadu_si512(staged.as_ptr().cast::<__m512i>());
        let shuffle = _mm512_loadu_si512(shuffle_mask.as_ptr().cast::<__m512i>());
        let lanes = _mm512_shuffle_epi8(input_vec, shuffle);

        let index0 = _mm512_and_si512(_mm512_srli_epi32(lanes, 18), _mm512_set1_epi32(0x0000_003f));
        let index1 = _mm512_and_si512(_mm512_srli_epi32(lanes, 4), _mm512_set1_epi32(0x0000_3f00));
        let index2 = _mm512_and_si512(_mm512_slli_epi32(lanes, 10), _mm512_set1_epi32(0x003f_0000));
        let index3 = _mm512_and_si512(_mm512_slli_epi32(lanes, 24), _mm512_set1_epi32(0x3f00_0000));
        let indices = _mm512_or_si512(
            _mm512_or_si512(index0, index1),
            _mm512_or_si512(index2, index3),
        );

        let table_vec = _mm512_loadu_si512(table.as_ptr().cast::<__m512i>());
        let encoded = _mm512_permutexvar_epi8(indices, table_vec);
        _mm512_storeu_si512(output.as_mut_ptr().cast::<__m512i>(), encoded);
        clear_zmm_registers_after_encode_block();
    }
    crate::wipe_bytes(&mut staged);
}

#[cfg(all(feature = "std", test))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn encode_24_bytes_avx2<A>(input: &[u8; 24], output: &mut [u8; 32])
where
    A: Alphabet,
{
    if !is_standard_or_url_safe_family::<A>() {
        scalar_encode_block::<A, 24, 32>(input, output);
        return;
    }

    // SAFETY: This function carries the AVX2 target-feature contract and the
    // fixed arrays satisfy the inner block's exact bounds.
    unsafe { encode_24_bytes_avx2_inner::<A>(input, output) };
}

#[inline]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "x86 loadu/storeu and loadl intrinsics accept unaligned pointers"
)]
#[target_feature(enable = "avx2")]
unsafe fn encode_24_bytes_avx2_inner<A>(input: &[u8; 24], output: &mut [u8; 32])
where
    A: Alphabet,
{
    // SAFETY: The fixed input array proves a 16-byte load at offset 0 and an
    // 8-byte load at offset 16 are in bounds. Lane construction discards the
    // four overlapping bytes and inserts zeros without reading past byte 23.
    // The fixed output array bounds the unaligned 32-byte store.
    unsafe {
        let first = _mm_loadu_si128(input.as_ptr().cast::<__m128i>());
        let tail = _mm_loadl_epi64(input.as_ptr().add(16).cast::<__m128i>());
        let lane0 = _mm_and_si128(first, _mm_set_epi32(0, -1, -1, -1));
        let lane1 = _mm_or_si128(_mm_srli_si128(first, 12), _mm_slli_si128(tail, 4));
        let input_vec = _mm256_inserti128_si256::<1>(_mm256_castsi128_si256(lane0), lane1);
        let shuffle = _mm256_setr_epi8(
            2, 1, 0, -128, 5, 4, 3, -128, 8, 7, 6, -128, 11, 10, 9, -128, 2, 1, 0, -128, 5, 4, 3,
            -128, 8, 7, 6, -128, 11, 10, 9, -128,
        );
        let shuffled = _mm256_shuffle_epi8(input_vec, shuffle);

        let index0 = _mm256_and_si256(
            _mm256_srli_epi32(shuffled, 18),
            _mm256_set1_epi32(0x0000_003f),
        );
        let index1 = _mm256_and_si256(
            _mm256_srli_epi32(shuffled, 4),
            _mm256_set1_epi32(0x0000_3f00),
        );
        let index2 = _mm256_and_si256(
            _mm256_slli_epi32(shuffled, 10),
            _mm256_set1_epi32(0x003f_0000),
        );
        let index3 = _mm256_and_si256(
            _mm256_slli_epi32(shuffled, 24),
            _mm256_set1_epi32(0x3f00_0000),
        );
        let indices = _mm256_or_si256(
            _mm256_or_si256(index0, index1),
            _mm256_or_si256(index2, index3),
        );

        let encoded = encode_standard_family_indices_avx2::<A>(indices);
        _mm256_storeu_si256(output.as_mut_ptr().cast::<__m256i>(), encoded);
    }
}

#[target_feature(enable = "avx2")]
unsafe fn encode_full_blocks_avx2<A>(input: &[u8], output: &mut [u8]) -> (usize, usize)
where
    A: Alphabet,
{
    let mut read = 0;
    let mut write = 0;
    while read + 24 <= input.len() {
        // SAFETY: The loop guards prove exact input/output blocks are within
        // the preflighted slices. The function carries the AVX2 contract.
        unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; 24]>());
            let encoded = &mut *(output.as_mut_ptr().add(write).cast::<[u8; 32]>());
            encode_24_bytes_avx2_inner::<A>(block, encoded);
        }
        read += 24;
        write += 32;
    }
    (read, write)
}

#[cfg(all(feature = "std", test))]
#[target_feature(enable = "ssse3,sse4.1")]
pub(crate) unsafe fn encode_12_bytes_ssse3_sse41<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    if !is_standard_or_url_safe_family::<A>() {
        scalar_encode_block::<A, 12, 16>(input, output);
        return;
    }
    // SAFETY: This function carries the SSSE3/SSE4.1 target-feature contract
    // and the fixed arrays satisfy the inner block's exact bounds.
    unsafe { encode_12_bytes_ssse3_sse41_inner::<A>(input, output) };
}

#[inline]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "x86 loadl/storeu intrinsics accept unaligned pointers"
)]
#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn encode_12_bytes_ssse3_sse41_inner<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    // SAFETY: The fixed input array proves an 8-byte load at offset 0 and an
    // unaligned 4-byte read at offset 8 are in bounds. Combining those values
    // creates a 12-byte vector with a zero high dword and does not over-read.
    // The fixed output array bounds the unaligned 16-byte store.
    unsafe {
        let low = _mm_loadl_epi64(input.as_ptr().cast::<__m128i>());
        let high_bits = core::ptr::read_unaligned(input.as_ptr().add(8).cast::<i32>());
        let input_vec = _mm_or_si128(low, _mm_slli_si128(_mm_cvtsi32_si128(high_bits), 8));
        let shuffle = _mm_setr_epi8(2, 1, 0, -128, 5, 4, 3, -128, 8, 7, 6, -128, 11, 10, 9, -128);
        let lanes = _mm_shuffle_epi8(input_vec, shuffle);

        let index0 = _mm_and_si128(_mm_srli_epi32(lanes, 18), _mm_set1_epi32(0x0000_003f));
        let index1 = _mm_and_si128(_mm_srli_epi32(lanes, 4), _mm_set1_epi32(0x0000_3f00));
        let index2 = _mm_and_si128(_mm_slli_epi32(lanes, 10), _mm_set1_epi32(0x003f_0000));
        let index3 = _mm_and_si128(_mm_slli_epi32(lanes, 24), _mm_set1_epi32(0x3f00_0000));
        let indices = _mm_or_si128(_mm_or_si128(index0, index1), _mm_or_si128(index2, index3));

        let encoded = encode_standard_family_indices_ssse3_sse41::<A>(indices);
        _mm_storeu_si128(output.as_mut_ptr().cast::<__m128i>(), encoded);
    }
}

#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn encode_full_blocks_ssse3_sse41<A>(input: &[u8], output: &mut [u8]) -> (usize, usize)
where
    A: Alphabet,
{
    let mut read = 0;
    let mut write = 0;
    while read + 12 <= input.len() {
        // SAFETY: The loop guards prove exact input/output blocks are within
        // the preflighted slices. This function carries the ISA contract.
        unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; 12]>());
            let encoded = &mut *(output.as_mut_ptr().add(write).cast::<[u8; 16]>());
            encode_12_bytes_ssse3_sse41_inner::<A>(block, encoded);
        }
        read += 12;
        write += 16;
    }
    (read, write)
}

pub(super) fn is_standard_or_url_safe_family<A>() -> bool
where
    A: Alphabet,
{
    const STANDARD_PREFIX: [u8; 62] =
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let mut index = 0;
    while index < STANDARD_PREFIX.len() {
        if A::ENCODE[index] != STANDARD_PREFIX[index] {
            return false;
        }
        index += 1;
    }

    (A::ENCODE[62] == b'+' && A::ENCODE[63] == b'/')
        || (A::ENCODE[62] == b'-' && A::ENCODE[63] == b'_')
}

fn scalar_encode_block<A, const IN: usize, const OUT: usize>(
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

        output[write] = encode_base64_value::<A>(b0 >> 2);
        output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
        output[write + 2] = encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
        output[write + 3] = encode_base64_value::<A>(b2 & 0b0011_1111);

        read += 3;
        write += 4;
    }
}

#[target_feature(enable = "sse4.1")]
unsafe fn encode_standard_family_indices_ssse3_sse41<A>(indices: __m128i) -> __m128i
where
    A: Alphabet,
{
    let offset62 = if A::ENCODE[62] == b'-' { -17 } else { -19 };
    let offset63 = if A::ENCODE[63] == b'_' { 32 } else { -16 };
    let lookup = _mm_setr_epi8(
        65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, offset62, offset63, 0, 0,
    );
    let reduced = _mm_subs_epu8(indices, _mm_set1_epi8(51));
    let upper_class = _mm_cmpgt_epi8(indices, _mm_set1_epi8(25));
    let lookup_index = _mm_sub_epi8(reduced, upper_class);
    let offset = _mm_shuffle_epi8(lookup, lookup_index);
    _mm_add_epi8(indices, offset)
}

#[target_feature(enable = "avx2")]
unsafe fn encode_standard_family_indices_avx2<A>(indices: __m256i) -> __m256i
where
    A: Alphabet,
{
    let offset62 = if A::ENCODE[62] == b'-' { -17 } else { -19 };
    let offset63 = if A::ENCODE[63] == b'_' { 32 } else { -16 };
    let lookup = _mm256_broadcastsi128_si256(_mm_setr_epi8(
        65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, offset62, offset63, 0, 0,
    ));
    let reduced = _mm256_subs_epu8(indices, _mm256_set1_epi8(51));
    let upper_class = _mm256_cmpgt_epi8(indices, _mm256_set1_epi8(25));
    let lookup_index = _mm256_sub_epi8(reduced, upper_class);
    let offset = _mm256_shuffle_epi8(lookup, lookup_index);
    _mm256_add_epi8(indices, offset)
}
