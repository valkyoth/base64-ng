#![allow(unsafe_code)]

use crate::Alphabet;

use core::arch::aarch64::{
    uint8x16_t, uint32x4_t, vaddq_u8, vandq_u8, vandq_u32, vbslq_u8, vceqq_u8, vcgeq_u8, vcltq_u8,
    vcombine_u8, vdup_n_u32, vdupq_n_u8, vdupq_n_u32, vget_low_u8, vgetq_lane_u32, vld1_u8,
    vld1q_u8, vminvq_u8, vorrq_u8, vorrq_u32, vqtbl1q_u8, vreinterpret_u8_u32,
    vreinterpretq_u8_u32, vreinterpretq_u32_u8, vshlq_n_u32, vshrq_n_u32, vst1_u8, vst1q_u8,
    vsubq_u8,
};

#[inline]
#[target_feature(enable = "neon")]
pub(super) unsafe fn encode_12_bytes<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    const SHUFFLE: [u8; 16] = [2, 1, 0, 255, 5, 4, 3, 255, 8, 7, 6, 255, 11, 10, 9, 255];

    // SAFETY: The first load reads exactly bytes 0..8. The unaligned scalar
    // read consumes exactly bytes 8..12 and is inserted into a zeroed high
    // half, so no 16-byte over-read occurs. The fixed output array bounds the
    // final vector store, and every alphabet index is masked to `0..=63`.
    unsafe {
        let low = vld1_u8(input.as_ptr());
        let tail_word = core::ptr::read_unaligned(input.as_ptr().add(8).cast::<u32>());
        let tail_words = core::arch::aarch64::vset_lane_u32::<0>(tail_word, vdup_n_u32(0));
        let packed = vcombine_u8(low, vreinterpret_u8_u32(tail_words));
        let shuffle = vld1q_u8(SHUFFLE.as_ptr());
        let lanes = vqtbl1q_u8(packed, shuffle);
        let lane_words: uint32x4_t = vreinterpretq_u32_u8(lanes);

        let index0 = vandq_u32(vshrq_n_u32(lane_words, 18), vdupq_n_u32(0x0000_003f));
        let index1 = vandq_u32(vshrq_n_u32(lane_words, 4), vdupq_n_u32(0x0000_3f00));
        let index2 = vandq_u32(vshlq_n_u32(lane_words, 10), vdupq_n_u32(0x003f_0000));
        let index3 = vandq_u32(vshlq_n_u32(lane_words, 24), vdupq_n_u32(0x3f00_0000));
        let indices = vreinterpretq_u8_u32(vorrq_u32(
            vorrq_u32(index0, index1),
            vorrq_u32(index2, index3),
        ));

        vst1q_u8(
            output.as_mut_ptr(),
            encode_standard_family_indices::<A>(indices),
        );
    }
}

#[inline]
#[target_feature(enable = "neon")]
pub(super) unsafe fn decode_16_bytes<A>(input: &[u8; 16], output: &mut [u8; 12]) -> bool
where
    A: Alphabet,
{
    const COMPACT: [u8; 16] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14, 255, 255, 255, 255];

    // SAFETY: Fixed arrays bound the exact 16-byte load and the exact 8+4
    // byte stores. Output is not touched unless all lanes classify as members
    // of the selected Standard-family alphabet. The target-feature contract
    // enables every intrinsic used below.
    unsafe {
        let ascii = vld1q_u8(input.as_ptr());
        let (values, valid) = map_ascii_to_values::<A>(ascii);
        if vminvq_u8(valid) != u8::MAX {
            return false;
        }

        let lanes: uint32x4_t = vreinterpretq_u32_u8(values);
        let byte0 = vorrq_u32(
            vshlq_n_u32(vandq_u32(lanes, vdupq_n_u32(0x0000_003f)), 2),
            vshrq_n_u32(vandq_u32(lanes, vdupq_n_u32(0x0000_3000)), 12),
        );
        let byte1 = vorrq_u32(
            vshlq_n_u32(vandq_u32(lanes, vdupq_n_u32(0x0000_0f00)), 4),
            vshrq_n_u32(vandq_u32(lanes, vdupq_n_u32(0x003c_0000)), 10),
        );
        let byte2 = vorrq_u32(
            vshlq_n_u32(vandq_u32(lanes, vdupq_n_u32(0x0003_0000)), 6),
            vshrq_n_u32(vandq_u32(lanes, vdupq_n_u32(0x3f00_0000)), 8),
        );
        let lane_bytes = vreinterpretq_u8_u32(vorrq_u32(vorrq_u32(byte0, byte1), byte2));
        let compact = vld1q_u8(COMPACT.as_ptr());
        let decoded = vqtbl1q_u8(lane_bytes, compact);

        vst1_u8(output.as_mut_ptr(), vget_low_u8(decoded));
        core::ptr::write_unaligned(
            output.as_mut_ptr().add(8).cast::<u32>(),
            vgetq_lane_u32::<2>(vreinterpretq_u32_u8(decoded)),
        );
    }
    true
}

#[inline]
#[target_feature(enable = "neon")]
unsafe fn map_ascii_to_values<A>(ascii: uint8x16_t) -> (uint8x16_t, uint8x16_t)
where
    A: Alphabet,
{
    let upper = range_mask(ascii, b'A', b'Z');
    let lower = range_mask(ascii, b'a', b'z');
    let digit = range_mask(ascii, b'0', b'9');
    let special62 = vceqq_u8(ascii, vdupq_n_u8(A::ENCODE[62]));
    let special63 = vceqq_u8(ascii, vdupq_n_u8(A::ENCODE[63]));
    let valid = or5(upper, lower, digit, special62, special63);

    let mut values = vdupq_n_u8(0);
    values = vbslq_u8(upper, vsubq_u8(ascii, vdupq_n_u8(b'A')), values);
    values = vbslq_u8(
        lower,
        vaddq_u8(vsubq_u8(ascii, vdupq_n_u8(b'a')), vdupq_n_u8(26)),
        values,
    );
    values = vbslq_u8(
        digit,
        vaddq_u8(vsubq_u8(ascii, vdupq_n_u8(b'0')), vdupq_n_u8(52)),
        values,
    );
    values = vbslq_u8(special62, vdupq_n_u8(62), values);
    values = vbslq_u8(special63, vdupq_n_u8(63), values);
    (values, valid)
}

#[inline]
#[target_feature(enable = "neon")]
fn range_mask(ascii: uint8x16_t, low: u8, high: u8) -> uint8x16_t {
    vandq_u8(
        vcgeq_u8(ascii, vdupq_n_u8(low)),
        vcltq_u8(ascii, vdupq_n_u8(high + 1)),
    )
}

#[inline]
#[target_feature(enable = "neon")]
fn or5(
    first: uint8x16_t,
    second: uint8x16_t,
    third: uint8x16_t,
    fourth: uint8x16_t,
    fifth: uint8x16_t,
) -> uint8x16_t {
    vorrq_u8(
        vorrq_u8(vorrq_u8(first, second), vorrq_u8(third, fourth)),
        fifth,
    )
}

#[inline]
#[target_feature(enable = "neon")]
unsafe fn encode_standard_family_indices<A>(indices: uint8x16_t) -> uint8x16_t
where
    A: Alphabet,
{
    let upper = vcltq_u8(indices, vdupq_n_u8(26));
    let lower = vandq_u8(
        vcgeq_u8(indices, vdupq_n_u8(26)),
        vcltq_u8(indices, vdupq_n_u8(52)),
    );
    let digit = vandq_u8(
        vcgeq_u8(indices, vdupq_n_u8(52)),
        vcltq_u8(indices, vdupq_n_u8(62)),
    );
    let special62 = vceqq_u8(indices, vdupq_n_u8(62));
    let special63 = vceqq_u8(indices, vdupq_n_u8(63));

    let mut encoded = vdupq_n_u8(0);
    encoded = vbslq_u8(upper, vaddq_u8(indices, vdupq_n_u8(b'A')), encoded);
    encoded = vbslq_u8(
        lower,
        vaddq_u8(vsubq_u8(indices, vdupq_n_u8(26)), vdupq_n_u8(b'a')),
        encoded,
    );
    encoded = vbslq_u8(
        digit,
        vaddq_u8(vsubq_u8(indices, vdupq_n_u8(52)), vdupq_n_u8(b'0')),
        encoded,
    );
    encoded = vbslq_u8(special62, vdupq_n_u8(A::ENCODE[62]), encoded);
    vbslq_u8(special63, vdupq_n_u8(A::ENCODE[63]), encoded)
}
