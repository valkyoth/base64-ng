#![allow(unsafe_code)]

use crate::{Alphabet, encode_base64_value};

#[cfg(target_arch = "x86")]
use core::arch::x86::{
    __m128i, __m256i, __m512i, _mm_add_epi8, _mm_and_si128, _mm_blendv_epi8, _mm_cmpeq_epi8,
    _mm_cmpgt_epi8, _mm_loadu_si128, _mm_or_si128, _mm_set1_epi8, _mm_set1_epi32, _mm_setr_epi8,
    _mm_shuffle_epi8, _mm_slli_epi32, _mm_srli_epi32, _mm_storeu_si128, _mm256_add_epi8,
    _mm256_and_si256, _mm256_blendv_epi8, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8, _mm256_loadu_si256,
    _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32, _mm256_setr_epi8, _mm256_shuffle_epi8,
    _mm256_slli_epi32, _mm256_srli_epi32, _mm256_storeu_si256, _mm512_and_si512,
    _mm512_loadu_si512, _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi32,
    _mm512_shuffle_epi8, _mm512_slli_epi32, _mm512_srli_epi32, _mm512_storeu_si512,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m128i, __m256i, __m512i, _mm_add_epi8, _mm_and_si128, _mm_blendv_epi8, _mm_cmpeq_epi8,
    _mm_cmpgt_epi8, _mm_loadu_si128, _mm_or_si128, _mm_set1_epi8, _mm_set1_epi32, _mm_setr_epi8,
    _mm_shuffle_epi8, _mm_slli_epi32, _mm_srli_epi32, _mm_storeu_si128, _mm256_add_epi8,
    _mm256_and_si256, _mm256_blendv_epi8, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8, _mm256_loadu_si256,
    _mm256_or_si256, _mm256_set1_epi8, _mm256_set1_epi32, _mm256_setr_epi8, _mm256_shuffle_epi8,
    _mm256_slli_epi32, _mm256_srli_epi32, _mm256_storeu_si256, _mm512_and_si512,
    _mm512_loadu_si512, _mm512_or_si512, _mm512_permutexvar_epi8, _mm512_set1_epi32,
    _mm512_shuffle_epi8, _mm512_slli_epi32, _mm512_srli_epi32, _mm512_storeu_si512,
};

#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm512_storeu_si512 accepts unaligned pointers"
)]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
pub(crate) unsafe fn encode_48_bytes_avx512<A>(input: &[u8; 48], output: &mut [u8; 64])
where
    A: Alphabet,
{
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
        clear_zmm_registers_for_test_prototype();
    }
    crate::wipe_bytes(&mut staged);
}

#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm256_storeu_si256 accepts unaligned pointers"
)]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn encode_24_bytes_avx2<A>(input: &[u8; 24], output: &mut [u8; 32])
where
    A: Alphabet,
{
    if !is_standard_or_url_safe_family::<A>() {
        scalar_encode_block::<A, 24, 32>(input, output);
        return;
    }

    let mut staged = [
        input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        input[9], input[10], input[11], 0, 0, 0, 0, input[12], input[13], input[14], input[15],
        input[16], input[17], input[18], input[19], input[20], input[21], input[22], input[23], 0,
        0, 0, 0,
    ];

    // SAFETY: `staged` and `output` are valid 32-byte arrays. The function's
    // target-feature contract enables AVX2. The load and store are unaligned
    // variants, and the shuffle mask uses `0x80` lanes only for zero-filled
    // bytes. The SIMD path is non-dispatchable test evidence.
    unsafe {
        let input_vec = _mm256_loadu_si256(staged.as_ptr().cast::<__m256i>());
        let shuffle = _mm256_setr_epi8(
            2, 1, 0, -128, 5, 4, 3, -128, 8, 7, 6, -128, 11, 10, 9, -128, 2, 1, 0, -128, 5, 4, 3,
            -128, 8, 7, 6, -128, 11, 10, 9, -128,
        );
        let lanes = _mm256_shuffle_epi8(input_vec, shuffle);

        let index0 = _mm256_and_si256(_mm256_srli_epi32(lanes, 18), _mm256_set1_epi32(0x0000_003f));
        let index1 = _mm256_and_si256(_mm256_srli_epi32(lanes, 4), _mm256_set1_epi32(0x0000_3f00));
        let index2 = _mm256_and_si256(_mm256_slli_epi32(lanes, 10), _mm256_set1_epi32(0x003f_0000));
        let index3 = _mm256_and_si256(_mm256_slli_epi32(lanes, 24), _mm256_set1_epi32(0x3f00_0000));
        let indices = _mm256_or_si256(
            _mm256_or_si256(index0, index1),
            _mm256_or_si256(index2, index3),
        );

        let encoded = encode_standard_family_indices_avx2::<A>(indices);
        _mm256_storeu_si256(output.as_mut_ptr().cast::<__m256i>(), encoded);
        clear_ymm_registers_for_test_prototype();
    }
    crate::wipe_bytes(&mut staged);
}

#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm_storeu_si128 accepts unaligned pointers"
)]
#[target_feature(enable = "ssse3,sse4.1")]
pub(crate) unsafe fn encode_12_bytes_ssse3_sse41<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    if !is_standard_or_url_safe_family::<A>() {
        scalar_encode_block::<A, 12, 16>(input, output);
        return;
    }

    let mut staged = [
        input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        input[9], input[10], input[11], 0, 0, 0, 0,
    ];

    // SAFETY: `staged` and `output` are valid 16-byte arrays. The function's
    // target-feature contract enables SSSE3/SSE4.1. The loads and stores are
    // unaligned variants, and the shuffle mask uses `0x80` lanes only for
    // zero-filled bytes. The SIMD path is non-dispatchable test evidence.
    unsafe {
        let input_vec = _mm_loadu_si128(staged.as_ptr().cast::<__m128i>());
        let shuffle = _mm_setr_epi8(2, 1, 0, -128, 5, 4, 3, -128, 8, 7, 6, -128, 11, 10, 9, -128);
        let lanes = _mm_shuffle_epi8(input_vec, shuffle);

        let index0 = _mm_and_si128(_mm_srli_epi32(lanes, 18), _mm_set1_epi32(0x0000_003f));
        let index1 = _mm_and_si128(_mm_srli_epi32(lanes, 4), _mm_set1_epi32(0x0000_3f00));
        let index2 = _mm_and_si128(_mm_slli_epi32(lanes, 10), _mm_set1_epi32(0x003f_0000));
        let index3 = _mm_and_si128(_mm_slli_epi32(lanes, 24), _mm_set1_epi32(0x3f00_0000));
        let indices = _mm_or_si128(_mm_or_si128(index0, index1), _mm_or_si128(index2, index3));

        let encoded = encode_standard_family_indices_ssse3_sse41::<A>(indices);
        _mm_storeu_si128(output.as_mut_ptr().cast::<__m128i>(), encoded);
        clear_xmm_registers_for_test_prototype();
    }
    crate::wipe_bytes(&mut staged);
}

fn is_standard_or_url_safe_family<A>() -> bool
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

    let lt26 = _mm_cmpgt_epi8(_mm_set1_epi8(26), indices);
    let lt52 = _mm_cmpgt_epi8(_mm_set1_epi8(52), indices);
    let lt62 = _mm_cmpgt_epi8(_mm_set1_epi8(62), indices);
    let eq62 = _mm_cmpeq_epi8(_mm_set1_epi8(62), indices);

    let mut offset = _mm_set1_epi8(offset63);
    offset = _mm_blendv_epi8(offset, _mm_set1_epi8(offset62), eq62);
    offset = _mm_blendv_epi8(offset, _mm_set1_epi8(-4), lt62);
    offset = _mm_blendv_epi8(offset, _mm_set1_epi8(71), lt52);
    offset = _mm_blendv_epi8(offset, _mm_set1_epi8(65), lt26);

    _mm_add_epi8(indices, offset)
}

#[target_feature(enable = "avx2")]
unsafe fn encode_standard_family_indices_avx2<A>(indices: __m256i) -> __m256i
where
    A: Alphabet,
{
    let offset62 = if A::ENCODE[62] == b'-' { -17 } else { -19 };
    let offset63 = if A::ENCODE[63] == b'_' { 32 } else { -16 };

    let lt26 = _mm256_cmpgt_epi8(_mm256_set1_epi8(26), indices);
    let lt52 = _mm256_cmpgt_epi8(_mm256_set1_epi8(52), indices);
    let lt62 = _mm256_cmpgt_epi8(_mm256_set1_epi8(62), indices);
    let eq62 = _mm256_cmpeq_epi8(_mm256_set1_epi8(62), indices);

    let mut offset = _mm256_set1_epi8(offset63);
    offset = _mm256_blendv_epi8(offset, _mm256_set1_epi8(offset62), eq62);
    offset = _mm256_blendv_epi8(offset, _mm256_set1_epi8(-4), lt62);
    offset = _mm256_blendv_epi8(offset, _mm256_set1_epi8(71), lt52);
    offset = _mm256_blendv_epi8(offset, _mm256_set1_epi8(65), lt26);

    _mm256_add_epi8(indices, offset)
}

unsafe fn clear_ymm_registers_for_test_prototype() {
    // SAFETY: The helper runs after the AVX2 prototype stores its output. The
    // XMM cleanup zeroes the lower halves declared to the compiler, and
    // `vzeroupper` clears upper YMM state before returning to scalar code.
    unsafe {
        clear_xmm_registers_for_test_prototype();
        core::arch::asm!("vzeroupper", options(nostack, preserves_flags, nomem));
    }
}

#[cfg(target_arch = "x86")]
unsafe fn clear_zmm_registers_for_test_prototype() {
    // SAFETY: This test-only cleanup runs after the prototype stores its
    // output. The explicit outputs tell the compiler these ZMM registers are
    // clobbered while the assembly clears them; `vzeroupper` clears upper
    // vector state before returning to scalar code.
    unsafe {
        core::arch::asm!(
            "vpxord zmm0, zmm0, zmm0",
            "vpxord zmm1, zmm1, zmm1",
            "vpxord zmm2, zmm2, zmm2",
            "vpxord zmm3, zmm3, zmm3",
            "vpxord zmm4, zmm4, zmm4",
            "vpxord zmm5, zmm5, zmm5",
            "vpxord zmm6, zmm6, zmm6",
            "vpxord zmm7, zmm7, zmm7",
            "vzeroupper",
            out("zmm0") _,
            out("zmm1") _,
            out("zmm2") _,
            out("zmm3") _,
            out("zmm4") _,
            out("zmm5") _,
            out("zmm6") _,
            out("zmm7") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn clear_zmm_registers_for_test_prototype() {
    // SAFETY: This test-only cleanup runs after the prototype stores its
    // output. The explicit outputs tell the compiler these ZMM registers are
    // clobbered while the assembly clears them; `vzeroupper` clears upper
    // vector state before returning to scalar code.
    unsafe {
        core::arch::asm!(
            "vpxord zmm0, zmm0, zmm0",
            "vpxord zmm1, zmm1, zmm1",
            "vpxord zmm2, zmm2, zmm2",
            "vpxord zmm3, zmm3, zmm3",
            "vpxord zmm4, zmm4, zmm4",
            "vpxord zmm5, zmm5, zmm5",
            "vpxord zmm6, zmm6, zmm6",
            "vpxord zmm7, zmm7, zmm7",
            "vpxord zmm8, zmm8, zmm8",
            "vpxord zmm9, zmm9, zmm9",
            "vpxord zmm10, zmm10, zmm10",
            "vpxord zmm11, zmm11, zmm11",
            "vpxord zmm12, zmm12, zmm12",
            "vpxord zmm13, zmm13, zmm13",
            "vpxord zmm14, zmm14, zmm14",
            "vpxord zmm15, zmm15, zmm15",
            "vpxord zmm16, zmm16, zmm16",
            "vpxord zmm17, zmm17, zmm17",
            "vpxord zmm18, zmm18, zmm18",
            "vpxord zmm19, zmm19, zmm19",
            "vpxord zmm20, zmm20, zmm20",
            "vpxord zmm21, zmm21, zmm21",
            "vpxord zmm22, zmm22, zmm22",
            "vpxord zmm23, zmm23, zmm23",
            "vpxord zmm24, zmm24, zmm24",
            "vpxord zmm25, zmm25, zmm25",
            "vpxord zmm26, zmm26, zmm26",
            "vpxord zmm27, zmm27, zmm27",
            "vpxord zmm28, zmm28, zmm28",
            "vpxord zmm29, zmm29, zmm29",
            "vpxord zmm30, zmm30, zmm30",
            "vpxord zmm31, zmm31, zmm31",
            "vzeroupper",
            out("zmm0") _,
            out("zmm1") _,
            out("zmm2") _,
            out("zmm3") _,
            out("zmm4") _,
            out("zmm5") _,
            out("zmm6") _,
            out("zmm7") _,
            out("zmm8") _,
            out("zmm9") _,
            out("zmm10") _,
            out("zmm11") _,
            out("zmm12") _,
            out("zmm13") _,
            out("zmm14") _,
            out("zmm15") _,
            out("zmm16") _,
            out("zmm17") _,
            out("zmm18") _,
            out("zmm19") _,
            out("zmm20") _,
            out("zmm21") _,
            out("zmm22") _,
            out("zmm23") _,
            out("zmm24") _,
            out("zmm25") _,
            out("zmm26") _,
            out("zmm27") _,
            out("zmm28") _,
            out("zmm29") _,
            out("zmm30") _,
            out("zmm31") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86")]
unsafe fn clear_xmm_registers_for_test_prototype() {
    // SAFETY: This test-only cleanup runs after the prototype stores its
    // output and before returning to scalar test code. The explicit outputs
    // tell the compiler these XMM registers are clobbered while the assembly
    // clears them to reduce register retention from the prototype path.
    unsafe {
        core::arch::asm!(
            "pxor xmm0, xmm0",
            "pxor xmm1, xmm1",
            "pxor xmm2, xmm2",
            "pxor xmm3, xmm3",
            "pxor xmm4, xmm4",
            "pxor xmm5, xmm5",
            "pxor xmm6, xmm6",
            "pxor xmm7, xmm7",
            out("xmm0") _,
            out("xmm1") _,
            out("xmm2") _,
            out("xmm3") _,
            out("xmm4") _,
            out("xmm5") _,
            out("xmm6") _,
            out("xmm7") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn clear_xmm_registers_for_test_prototype() {
    // SAFETY: This test-only cleanup runs after the prototype stores its
    // output and before returning to scalar test code. The explicit outputs
    // tell the compiler these XMM registers are clobbered while the assembly
    // clears them to reduce register retention from the prototype path.
    unsafe {
        core::arch::asm!(
            "pxor xmm0, xmm0",
            "pxor xmm1, xmm1",
            "pxor xmm2, xmm2",
            "pxor xmm3, xmm3",
            "pxor xmm4, xmm4",
            "pxor xmm5, xmm5",
            "pxor xmm6, xmm6",
            "pxor xmm7, xmm7",
            "pxor xmm8, xmm8",
            "pxor xmm9, xmm9",
            "pxor xmm10, xmm10",
            "pxor xmm11, xmm11",
            "pxor xmm12, xmm12",
            "pxor xmm13, xmm13",
            "pxor xmm14, xmm14",
            "pxor xmm15, xmm15",
            out("xmm0") _,
            out("xmm1") _,
            out("xmm2") _,
            out("xmm3") _,
            out("xmm4") _,
            out("xmm5") _,
            out("xmm6") _,
            out("xmm7") _,
            out("xmm8") _,
            out("xmm9") _,
            out("xmm10") _,
            out("xmm11") _,
            out("xmm12") _,
            out("xmm13") _,
            out("xmm14") _,
            out("xmm15") _,
            options(nostack, preserves_flags)
        );
    }
}
