#![allow(unsafe_code)]

mod cleanup;
mod decode;
mod decode_direct;

#[cfg(all(feature = "std", test))]
use crate::encode_base64_value;
use crate::{Alphabet, EncodeError, checked_encoded_len, scalar};

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

pub(crate) fn avx2_encode_available() -> bool {
    super::avx2_available()
}

pub(crate) fn ssse3_sse41_encode_available() -> bool {
    super::ssse3_sse41_available()
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

    // SAFETY: Health admission proves the complete AVX-512 VBMI feature
    // bundle. The output preflight and fixed block ratio prove every masked
    // load and direct store remains in bounds.
    let (read, write) = unsafe { encode_full_blocks_avx512::<A>(input, output) };

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
    __m128i, __m256i, __m512i, __mmask64, _mm_add_epi8, _mm_and_si128, _mm_cmpgt_epi8,
    _mm_cvtsi32_si128, _mm_loadl_epi64, _mm_loadu_si128, _mm_or_si128, _mm_set_epi32,
    _mm_set1_epi8, _mm_set1_epi32, _mm_setr_epi8, _mm_shuffle_epi8, _mm_slli_epi32, _mm_slli_si128,
    _mm_srli_epi32, _mm_srli_si128, _mm_storeu_si128, _mm_sub_epi8, _mm_subs_epu8, _mm256_add_epi8,
    _mm256_and_si256, _mm256_broadcastsi128_si256, _mm256_castsi128_si256, _mm256_cmpgt_epi8,
    _mm256_inserti128_si256, _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32,
    _mm256_setr_epi8, _mm256_shuffle_epi8, _mm256_slli_epi32, _mm256_srli_epi32,
    _mm256_storeu_si256, _mm256_sub_epi8, _mm256_subs_epu8, _mm512_and_si512, _mm512_loadu_si512,
    _mm512_maskz_loadu_epi8, _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi32,
    _mm512_slli_epi32, _mm512_srli_epi32, _mm512_storeu_si512,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m128i, __m256i, __m512i, __mmask64, _mm_add_epi8, _mm_and_si128, _mm_cmpgt_epi8,
    _mm_cvtsi32_si128, _mm_loadl_epi64, _mm_loadu_si128, _mm_or_si128, _mm_set_epi32,
    _mm_set1_epi8, _mm_set1_epi32, _mm_setr_epi8, _mm_shuffle_epi8, _mm_slli_epi32, _mm_slli_si128,
    _mm_srli_epi32, _mm_srli_si128, _mm_storeu_si128, _mm_sub_epi8, _mm_subs_epu8, _mm256_add_epi8,
    _mm256_and_si256, _mm256_broadcastsi128_si256, _mm256_castsi128_si256, _mm256_cmpgt_epi8,
    _mm256_inserti128_si256, _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32,
    _mm256_setr_epi8, _mm256_shuffle_epi8, _mm256_slli_epi32, _mm256_srli_epi32,
    _mm256_storeu_si256, _mm256_sub_epi8, _mm256_subs_epu8, _mm512_and_si512, _mm512_loadu_si512,
    _mm512_maskz_loadu_epi8, _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi32,
    _mm512_slli_epi32, _mm512_srli_epi32, _mm512_storeu_si512,
};

#[cfg(all(feature = "std", test))]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
pub(crate) unsafe fn encode_48_bytes_avx512<A>(input: &[u8; 48], output: &mut [u8; 64])
where
    A: Alphabet,
{
    if !is_standard_or_url_safe_family::<A>() {
        scalar_encode_block::<A, 48, 64>(input, output);
        return;
    }

    // SAFETY: This function carries the complete target-feature contract and
    // fixed arrays satisfy the exact inner block bounds.
    unsafe {
        encode_48_bytes_avx512_inner::<A>(input, output);
        clear_zmm_registers_after_encode_block();
    }
}

#[inline]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "AVX-512 masked load and store intrinsics accept unaligned pointers"
)]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
unsafe fn encode_48_bytes_avx512_inner<A>(input: &[u8; 48], output: &mut [u8; 64])
where
    A: Alphabet,
{
    const INPUT_BYTES: __mmask64 = 0x0000_ffff_ffff_ffff;
    const EXPAND_TO_LANES: [u8; 64] = [
        2, 1, 0, 0, 5, 4, 3, 0, 8, 7, 6, 0, 11, 10, 9, 0, 14, 13, 12, 0, 17, 16, 15, 0, 20, 19, 18,
        0, 23, 22, 21, 0, 26, 25, 24, 0, 29, 28, 27, 0, 32, 31, 30, 0, 35, 34, 33, 0, 38, 37, 36,
        0, 41, 40, 39, 0, 44, 43, 42, 0, 47, 46, 45, 0,
    ];
    let table = A::ENCODE;

    // SAFETY: The mask activates exactly the 48 bytes present in `input`, so
    // the masked load cannot read beyond the fixed array. Every byte-permute
    // index is in `0..=47`, all extracted alphabet indices are in `0..=63`,
    // and the fixed output array bounds the unaligned 64-byte store.
    unsafe {
        let packed = _mm512_maskz_loadu_epi8(INPUT_BYTES, input.as_ptr().cast::<i8>());
        let expand = _mm512_loadu_si512(EXPAND_TO_LANES.as_ptr().cast::<__m512i>());
        let lanes = _mm512_permutexvar_epi8(expand, packed);

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
    }
}

#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
unsafe fn encode_full_blocks_avx512<A>(input: &[u8], output: &mut [u8]) -> (usize, usize)
where
    A: Alphabet,
{
    let mut read = 0;
    let mut write = 0;
    while read + 48 <= input.len() {
        // SAFETY: The loop guards prove exact fixed blocks are within the
        // preflighted slices. This function carries the complete ISA contract.
        unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; 48]>());
            let encoded = &mut *(output.as_mut_ptr().add(write).cast::<[u8; 64]>());
            encode_48_bytes_avx512_inner::<A>(block, encoded);
        }
        read += 48;
        write += 64;
    }
    // SAFETY: At least one block was processed because the caller rejects
    // inputs shorter than 48 bytes. No vector value remains live afterward.
    unsafe { clear_zmm_registers_after_encode_block() };
    (read, write)
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

#[cfg(all(feature = "std", test))]
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
