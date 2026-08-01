#![allow(unsafe_code)]

use crate::Alphabet;

#[cfg(target_arch = "x86")]
use core::arch::x86::{
    __m128i, __m256i, __m512i, __mmask64, _mm_add_epi8, _mm_and_si128, _mm_cmpeq_epi8,
    _mm_cmpgt_epi8, _mm_cvtsi128_si32, _mm_loadu_si128, _mm_madd_epi16, _mm_maddubs_epi16,
    _mm_movemask_epi8, _mm_or_si128, _mm_set1_epi8, _mm_set1_epi32, _mm_setr_epi8,
    _mm_shuffle_epi8, _mm_srli_si128, _mm_storel_epi64, _mm_sub_epi8, _mm256_add_epi8,
    _mm256_and_si256, _mm256_castsi256_si128, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8,
    _mm256_extracti128_si256, _mm256_loadu_si256, _mm256_madd_epi16, _mm256_maddubs_epi16,
    _mm256_movemask_epi8, _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32, _mm256_setr_epi8,
    _mm256_shuffle_epi8, _mm256_sub_epi8, _mm512_cmpeq_epi8_mask, _mm512_cmpge_epu8_mask,
    _mm512_cmple_epu8_mask, _mm512_loadu_si512, _mm512_madd_epi16, _mm512_maddubs_epi16,
    _mm512_mask_storeu_epi8, _mm512_maskz_add_epi8, _mm512_maskz_set1_epi8, _mm512_maskz_sub_epi8,
    _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi8, _mm512_set1_epi32,
    _mm512_shuffle_epi8,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m128i, __m256i, __m512i, __mmask64, _mm_add_epi8, _mm_and_si128, _mm_cmpeq_epi8,
    _mm_cmpgt_epi8, _mm_cvtsi128_si32, _mm_loadu_si128, _mm_madd_epi16, _mm_maddubs_epi16,
    _mm_movemask_epi8, _mm_or_si128, _mm_set1_epi8, _mm_set1_epi32, _mm_setr_epi8,
    _mm_shuffle_epi8, _mm_srli_si128, _mm_storel_epi64, _mm_sub_epi8, _mm256_add_epi8,
    _mm256_and_si256, _mm256_castsi256_si128, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8,
    _mm256_extracti128_si256, _mm256_loadu_si256, _mm256_madd_epi16, _mm256_maddubs_epi16,
    _mm256_movemask_epi8, _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32, _mm256_setr_epi8,
    _mm256_shuffle_epi8, _mm256_sub_epi8, _mm512_cmpeq_epi8_mask, _mm512_cmpge_epu8_mask,
    _mm512_cmple_epu8_mask, _mm512_loadu_si512, _mm512_madd_epi16, _mm512_maddubs_epi16,
    _mm512_mask_storeu_epi8, _mm512_maskz_add_epi8, _mm512_maskz_set1_epi8, _mm512_maskz_sub_epi8,
    _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi8, _mm512_set1_epi32,
    _mm512_shuffle_epi8,
};

#[inline]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "AVX-512 load and masked-store intrinsics accept unaligned pointers"
)]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
pub(super) unsafe fn decode_64_bytes_avx512<A>(input: &[u8; 64], output: &mut [u8; 48]) -> bool
where
    A: Alphabet,
{
    const OUTPUT_BYTES: __mmask64 = 0x0000_ffff_ffff_ffff;
    const SHUFFLE: [u8; 64] = [
        2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, 0x80, 0x80, 0x80, 0x80, 2, 1, 0, 6, 5, 4, 10, 9, 8,
        14, 13, 12, 0x80, 0x80, 0x80, 0x80, 2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, 0x80, 0x80,
        0x80, 0x80, 2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, 0x80, 0x80, 0x80, 0x80,
    ];
    const COMPACT: [u8; 64] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 32,
        33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    // SAFETY: The target-feature contract enables every intrinsic. Fixed
    // arrays provide exact load and masked-store bounds, and output is written
    // only after all 64 bytes belong to the selected Standard-family alphabet.
    unsafe {
        let ascii = _mm512_loadu_si512(input.as_ptr().cast::<__m512i>());
        let (values, valid) = map_ascii_to_values_avx512::<A>(ascii);
        if valid != u64::MAX {
            return false;
        }
        let merged_pairs = _mm512_maddubs_epi16(values, _mm512_set1_epi32(0x0140_0140));
        let merged_quads = _mm512_madd_epi16(merged_pairs, _mm512_set1_epi32(0x0001_1000));
        let shuffle = _mm512_loadu_si512(SHUFFLE.as_ptr().cast::<__m512i>());
        let decoded_lanes = _mm512_shuffle_epi8(merged_quads, shuffle);
        let compact = _mm512_loadu_si512(COMPACT.as_ptr().cast::<__m512i>());
        let decoded = _mm512_permutexvar_epi8(compact, decoded_lanes);
        _mm512_mask_storeu_epi8(output.as_mut_ptr().cast::<i8>(), OUTPUT_BYTES, decoded);
    }
    true
}

#[inline]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "_mm_loadu_si128 accepts an unaligned pointer"
)]
#[target_feature(enable = "ssse3,sse4.1")]
pub(super) unsafe fn decode_16_bytes_ssse3_sse41<A>(input: &[u8; 16], output: &mut [u8; 12]) -> bool
where
    A: Alphabet,
{
    // SAFETY: The target-feature contract enables each intrinsic. The fixed
    // arrays provide exact load/store bounds, and output is written only after
    // every byte belongs to the selected Standard-family alphabet.
    unsafe {
        let ascii = _mm_loadu_si128(input.as_ptr().cast::<__m128i>());
        let (values, valid) = map_ascii_to_values_ssse3::<A>(ascii);
        if _mm_movemask_epi8(valid) != 0xffff {
            return false;
        }
        let merged_pairs = _mm_maddubs_epi16(values, _mm_set1_epi32(0x0140_0140));
        let merged_quads = _mm_madd_epi16(merged_pairs, _mm_set1_epi32(0x0001_1000));
        let shuffle = _mm_setr_epi8(
            2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -128, -128, -128, -128,
        );
        store_12_bytes(_mm_shuffle_epi8(merged_quads, shuffle), output.as_mut_ptr());
    }
    true
}

#[inline]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "_mm256_loadu_si256 accepts an unaligned pointer"
)]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn decode_32_bytes_avx2<A>(input: &[u8; 32], output: &mut [u8; 24]) -> bool
where
    A: Alphabet,
{
    // SAFETY: The target-feature contract enables each intrinsic. The fixed
    // arrays provide exact load/store bounds, and output is written only after
    // every byte belongs to the selected Standard-family alphabet.
    unsafe {
        let ascii = _mm256_loadu_si256(input.as_ptr().cast::<__m256i>());
        let (values, valid) = map_ascii_to_values_avx2::<A>(ascii);
        if _mm256_movemask_epi8(valid) != -1 {
            return false;
        }
        let merged_pairs = _mm256_maddubs_epi16(values, _mm256_set1_epi32(0x0140_0140));
        let merged_quads = _mm256_madd_epi16(merged_pairs, _mm256_set1_epi32(0x0001_1000));
        let shuffle = _mm256_setr_epi8(
            2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -128, -128, -128, -128, 2, 1, 0, 6, 5, 4, 10,
            9, 8, 14, 13, 12, -128, -128, -128, -128,
        );
        let decoded = _mm256_shuffle_epi8(merged_quads, shuffle);
        store_12_bytes(_mm256_castsi256_si128(decoded), output.as_mut_ptr());
        store_12_bytes(
            _mm256_extracti128_si256::<1>(decoded),
            output.as_mut_ptr().add(12),
        );
    }
    true
}

#[inline]
#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn map_ascii_to_values_ssse3<A>(ascii: __m128i) -> (__m128i, __m128i)
where
    A: Alphabet,
{
    // SAFETY: The caller carries the target-feature contract.
    unsafe {
        let upper = range_mask_ssse3(ascii, b'A', b'Z');
        let lower = range_mask_ssse3(ascii, b'a', b'z');
        let digit = range_mask_ssse3(ascii, b'0', b'9');
        let special62 = _mm_cmpeq_epi8(ascii, _mm_set1_epi8(ascii_lane(A::ENCODE[62])));
        let special63 = _mm_cmpeq_epi8(ascii, _mm_set1_epi8(ascii_lane(A::ENCODE[63])));
        let valid = or5_ssse3(upper, lower, digit, special62, special63);
        let values = or5_ssse3(
            _mm_and_si128(upper, _mm_sub_epi8(ascii, _mm_set1_epi8(ascii_lane(b'A')))),
            _mm_and_si128(
                lower,
                _mm_add_epi8(
                    _mm_sub_epi8(ascii, _mm_set1_epi8(ascii_lane(b'a'))),
                    _mm_set1_epi8(26),
                ),
            ),
            _mm_and_si128(
                digit,
                _mm_add_epi8(
                    _mm_sub_epi8(ascii, _mm_set1_epi8(ascii_lane(b'0'))),
                    _mm_set1_epi8(52),
                ),
            ),
            _mm_and_si128(special62, _mm_set1_epi8(62)),
            _mm_and_si128(special63, _mm_set1_epi8(63)),
        );
        (values, valid)
    }
}

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn map_ascii_to_values_avx2<A>(ascii: __m256i) -> (__m256i, __m256i)
where
    A: Alphabet,
{
    // SAFETY: The caller carries the target-feature contract.
    unsafe {
        let upper = range_mask_avx2(ascii, b'A', b'Z');
        let lower = range_mask_avx2(ascii, b'a', b'z');
        let digit = range_mask_avx2(ascii, b'0', b'9');
        let special62 = _mm256_cmpeq_epi8(ascii, _mm256_set1_epi8(ascii_lane(A::ENCODE[62])));
        let special63 = _mm256_cmpeq_epi8(ascii, _mm256_set1_epi8(ascii_lane(A::ENCODE[63])));
        let valid = or5_avx2(upper, lower, digit, special62, special63);
        let values = or5_avx2(
            _mm256_and_si256(
                upper,
                _mm256_sub_epi8(ascii, _mm256_set1_epi8(ascii_lane(b'A'))),
            ),
            _mm256_and_si256(
                lower,
                _mm256_add_epi8(
                    _mm256_sub_epi8(ascii, _mm256_set1_epi8(ascii_lane(b'a'))),
                    _mm256_set1_epi8(26),
                ),
            ),
            _mm256_and_si256(
                digit,
                _mm256_add_epi8(
                    _mm256_sub_epi8(ascii, _mm256_set1_epi8(ascii_lane(b'0'))),
                    _mm256_set1_epi8(52),
                ),
            ),
            _mm256_and_si256(special62, _mm256_set1_epi8(62)),
            _mm256_and_si256(special63, _mm256_set1_epi8(63)),
        );
        (values, valid)
    }
}

#[inline]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
unsafe fn map_ascii_to_values_avx512<A>(ascii: __m512i) -> (__m512i, __mmask64)
where
    A: Alphabet,
{
    // SAFETY: The caller carries the complete AVX-512 feature contract.
    unsafe {
        let upper = range_mask_avx512(ascii, b'A', b'Z');
        let lower = range_mask_avx512(ascii, b'a', b'z');
        let digit = range_mask_avx512(ascii, b'0', b'9');
        let special62 = _mm512_cmpeq_epi8_mask(ascii, _mm512_set1_epi8(ascii_lane(A::ENCODE[62])));
        let special63 = _mm512_cmpeq_epi8_mask(ascii, _mm512_set1_epi8(ascii_lane(A::ENCODE[63])));
        let valid = upper | lower | digit | special62 | special63;
        let upper_values = _mm512_maskz_sub_epi8(upper, ascii, _mm512_set1_epi8(ascii_lane(b'A')));
        let lower_values = _mm512_maskz_add_epi8(
            lower,
            _mm512_maskz_sub_epi8(lower, ascii, _mm512_set1_epi8(ascii_lane(b'a'))),
            _mm512_set1_epi8(26),
        );
        let digit_values = _mm512_maskz_add_epi8(
            digit,
            _mm512_maskz_sub_epi8(digit, ascii, _mm512_set1_epi8(ascii_lane(b'0'))),
            _mm512_set1_epi8(52),
        );
        let values = or5_avx512(
            upper_values,
            lower_values,
            digit_values,
            _mm512_maskz_set1_epi8(special62, 62),
            _mm512_maskz_set1_epi8(special63, 63),
        );
        (values, valid)
    }
}

#[inline]
#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn range_mask_ssse3(ascii: __m128i, low: u8, high: u8) -> __m128i {
    _mm_and_si128(
        _mm_cmpgt_epi8(ascii, _mm_set1_epi8(ascii_lane(low - 1))),
        _mm_cmpgt_epi8(_mm_set1_epi8(ascii_lane(high + 1)), ascii),
    )
}

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn range_mask_avx2(ascii: __m256i, low: u8, high: u8) -> __m256i {
    _mm256_and_si256(
        _mm256_cmpgt_epi8(ascii, _mm256_set1_epi8(ascii_lane(low - 1))),
        _mm256_cmpgt_epi8(_mm256_set1_epi8(ascii_lane(high + 1)), ascii),
    )
}

#[inline]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
unsafe fn range_mask_avx512(ascii: __m512i, low: u8, high: u8) -> __mmask64 {
    _mm512_cmpge_epu8_mask(ascii, _mm512_set1_epi8(ascii_lane(low)))
        & _mm512_cmple_epu8_mask(ascii, _mm512_set1_epi8(ascii_lane(high)))
}

#[inline]
#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn or5_ssse3(
    first: __m128i,
    second: __m128i,
    third: __m128i,
    fourth: __m128i,
    fifth: __m128i,
) -> __m128i {
    _mm_or_si128(
        _mm_or_si128(_mm_or_si128(first, second), _mm_or_si128(third, fourth)),
        fifth,
    )
}

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn or5_avx2(
    first: __m256i,
    second: __m256i,
    third: __m256i,
    fourth: __m256i,
    fifth: __m256i,
) -> __m256i {
    _mm256_or_si256(
        _mm256_or_si256(
            _mm256_or_si256(first, second),
            _mm256_or_si256(third, fourth),
        ),
        fifth,
    )
}

#[inline]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
unsafe fn or5_avx512(
    first: __m512i,
    second: __m512i,
    third: __m512i,
    fourth: __m512i,
    fifth: __m512i,
) -> __m512i {
    _mm512_or_si512(
        _mm512_or_si512(
            _mm512_or_si512(first, second),
            _mm512_or_si512(third, fourth),
        ),
        fifth,
    )
}

#[inline]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "_mm_storel_epi64 accepts an unaligned pointer"
)]
#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn store_12_bytes(decoded: __m128i, output: *mut u8) {
    // SAFETY: Callers provide at least 12 writable bytes. The first store
    // writes 8 bytes and the unaligned scalar store writes the final 4.
    unsafe {
        _mm_storel_epi64(output.cast::<__m128i>(), decoded);
        core::ptr::write_unaligned(
            output.add(8).cast::<i32>(),
            _mm_cvtsi128_si32(_mm_srli_si128::<8>(decoded)),
        );
    }
}

#[inline]
#[allow(
    clippy::cast_possible_wrap,
    reason = "all caller-provided Base64 alphabet bytes are validated ASCII"
)]
const fn ascii_lane(byte: u8) -> i8 {
    byte as i8
}
