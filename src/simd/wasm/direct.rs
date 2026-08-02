#![allow(unsafe_code)]

use core::arch::wasm32::{
    i8x16_bitmask, u8x16_add, u8x16_eq, u8x16_ge, u8x16_lt, u8x16_shuffle, u8x16_splat, u8x16_sub,
    u32x4_shl, u32x4_shr, u32x4_splat, v128, v128_and, v128_bitselect, v128_load32_lane,
    v128_load64_zero, v128_or, v128_store, v128_store32_lane, v128_store64_lane,
};

use crate::Alphabet;

#[target_feature(enable = "simd128")]
pub(super) unsafe fn encode_12_bytes<A: Alphabet>(input: &[u8; 12], output: &mut [u8; 16]) {
    // SAFETY: The fixed input has eight bytes at offset zero and four at offset
    // eight. The target-feature contract enables every simd128 operation.
    let input_vec = unsafe {
        let low = v128_load64_zero(input.as_ptr().cast());
        v128_load32_lane::<2>(low, input.as_ptr().add(8).cast())
    };
    let lanes =
        u8x16_shuffle::<2, 1, 0, 16, 5, 4, 3, 16, 8, 7, 6, 16, 11, 10, 9, 16>(input_vec, input_vec);
    let index0 = v128_and(u32x4_shr(lanes, 18), u32x4_splat(0x0000_003f));
    let index1 = v128_and(u32x4_shr(lanes, 4), u32x4_splat(0x0000_3f00));
    let index2 = v128_and(u32x4_shl(lanes, 10), u32x4_splat(0x003f_0000));
    let index3 = v128_and(u32x4_shl(lanes, 24), u32x4_splat(0x3f00_0000));
    let indices = v128_or(v128_or(index0, index1), v128_or(index2, index3));
    let encoded = encode_indices::<A>(indices);

    // SAFETY: The output array is exactly one v128 wide.
    unsafe { v128_store(output.as_mut_ptr().cast(), encoded) };
}

#[target_feature(enable = "simd128")]
pub(super) unsafe fn decode_16_bytes<A: Alphabet>(input: &[u8; 16], output: &mut [u8; 12]) -> bool {
    // SAFETY: The input array is exactly one v128 wide.
    let ascii = unsafe { core::arch::wasm32::v128_load(input.as_ptr().cast()) };
    let upper = v128_and(
        u8x16_ge(ascii, u8x16_splat(b'A')),
        u8x16_lt(ascii, u8x16_splat(b'Z' + 1)),
    );
    let lower = v128_and(
        u8x16_ge(ascii, u8x16_splat(b'a')),
        u8x16_lt(ascii, u8x16_splat(b'z' + 1)),
    );
    let digit = v128_and(
        u8x16_ge(ascii, u8x16_splat(b'0')),
        u8x16_lt(ascii, u8x16_splat(b'9' + 1)),
    );
    let symbol62 = u8x16_eq(ascii, u8x16_splat(A::ENCODE[62]));
    let symbol63 = u8x16_eq(ascii, u8x16_splat(A::ENCODE[63]));
    let valid = v128_or(
        v128_or(upper, lower),
        v128_or(digit, v128_or(symbol62, symbol63)),
    );
    if i8x16_bitmask(valid) != 0xffff {
        return false;
    }

    let mut values = u8x16_splat(0);
    values = v128_bitselect(u8x16_sub(ascii, u8x16_splat(b'A')), values, upper);
    values = v128_bitselect(
        u8x16_add(u8x16_sub(ascii, u8x16_splat(b'a')), u8x16_splat(26)),
        values,
        lower,
    );
    values = v128_bitselect(
        u8x16_add(u8x16_sub(ascii, u8x16_splat(b'0')), u8x16_splat(52)),
        values,
        digit,
    );
    values = v128_bitselect(u8x16_splat(62), values, symbol62);
    values = v128_bitselect(u8x16_splat(63), values, symbol63);

    let byte0 = v128_or(
        v128_and(u32x4_shl(values, 2), u32x4_splat(0x0000_00fc)),
        v128_and(u32x4_shr(values, 12), u32x4_splat(0x0000_0003)),
    );
    let byte1 = v128_or(
        v128_and(u32x4_shl(values, 4), u32x4_splat(0x0000_f000)),
        v128_and(u32x4_shr(values, 10), u32x4_splat(0x0000_0f00)),
    );
    let byte2 = v128_or(
        v128_and(u32x4_shl(values, 6), u32x4_splat(0x00c0_0000)),
        v128_and(u32x4_shr(values, 8), u32x4_splat(0x003f_0000)),
    );
    let merged = v128_or(byte0, v128_or(byte1, byte2));
    let compact =
        u8x16_shuffle::<0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14, 15, 15, 15, 15>(merged, merged);

    // SAFETY: These lane stores write exactly eight plus four bytes into the
    // fixed 12-byte output and occur only after all lanes are classified.
    unsafe {
        v128_store64_lane::<0>(compact, output.as_mut_ptr().cast());
        v128_store32_lane::<2>(compact, output.as_mut_ptr().add(8).cast());
    }
    true
}

#[target_feature(enable = "simd128")]
fn encode_indices<A: Alphabet>(indices: v128) -> v128 {
    let upper = u8x16_lt(indices, u8x16_splat(26));
    let lower = v128_and(
        u8x16_ge(indices, u8x16_splat(26)),
        u8x16_lt(indices, u8x16_splat(52)),
    );
    let digit = v128_and(
        u8x16_ge(indices, u8x16_splat(52)),
        u8x16_lt(indices, u8x16_splat(62)),
    );
    let symbol62 = u8x16_eq(indices, u8x16_splat(62));
    let symbol63 = u8x16_eq(indices, u8x16_splat(63));

    let mut encoded = u8x16_splat(0);
    encoded = v128_bitselect(u8x16_add(indices, u8x16_splat(b'A')), encoded, upper);
    encoded = v128_bitselect(
        u8x16_add(u8x16_sub(indices, u8x16_splat(26)), u8x16_splat(b'a')),
        encoded,
        lower,
    );
    encoded = v128_bitselect(
        u8x16_add(u8x16_sub(indices, u8x16_splat(52)), u8x16_splat(b'0')),
        encoded,
        digit,
    );
    encoded = v128_bitselect(u8x16_splat(A::ENCODE[62]), encoded, symbol62);
    v128_bitselect(u8x16_splat(A::ENCODE[63]), encoded, symbol63)
}
