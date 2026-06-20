use core::arch::wasm32::{
    u8x16_add, u8x16_eq, u8x16_ge, u8x16_lt, u8x16_shuffle, u8x16_splat, u8x16_sub, u32x4_shl,
    u32x4_shr, u32x4_splat, v128, v128_and, v128_bitselect, v128_load, v128_or, v128_store,
};

use crate::{Alphabet, Engine, Standard, UrlSafe, decode_alphabet_byte, encode_base64_value};

/// Encodes one 12-byte block into 16 bytes through the inactive wasm
/// `simd128` prototype.
///
/// This is not an admitted fast path. It exists to compile and review wasm
/// vector encode code without changing runtime dispatch. Standard and URL-safe
/// alphabets use real fixed-block `simd128` logic. Custom alphabets use the
/// scalar fallback scaffold because portable wasm SIMD lacks a direct 64-byte
/// table lookup instruction.
///
/// Admission note: wasm `simd128` has a second optimization layer in the
/// runtime/JIT. This prototype is codegen evidence only and does not claim a
/// runtime timing, register-retention, or JIT zeroization guarantee.
///
/// # Safety
///
/// The caller must execute this function only when `simd128` is available for
/// the current wasm runtime. The input and output sizes are fixed by their
/// array types.
#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[target_feature(enable = "simd128")]
unsafe fn encode_12_bytes_wasm_simd128<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    if is_standard_or_url_safe_family::<A>() {
        // SAFETY: The caller has proven simd128 availability. The helper uses
        // fixed input/output arrays and supports this alphabet family.
        unsafe {
            encode_12_bytes_wasm_standard_family::<A>(input, output);
        }
        return;
    }

    scalar_encode_block::<A>(input, output);
}

fn scalar_encode_block<A>(input: &[u8; 12], output: &mut [u8; 16])
where
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

fn is_standard_or_url_safe_family<A>() -> bool
where
    A: Alphabet,
{
    let encode = A::ENCODE;
    let mut index = 0;
    while index < 62 {
        if encode[index] != Standard::ENCODE[index] {
            return false;
        }
        index += 1;
    }

    (encode[62] == b'+' && encode[63] == b'/') || (encode[62] == b'-' && encode[63] == b'_')
}

#[target_feature(enable = "simd128")]
unsafe fn encode_12_bytes_wasm_standard_family<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    let mut staged = [
        input[0], input[1], input[2], 0, input[3], input[4], input[5], 0, input[6], input[7],
        input[8], 0, input[9], input[10], input[11], 0,
    ];

    // SAFETY: Fixed arrays back every 128-bit load/store, the target-feature
    // contract enables wasm simd128, and shuffle zero lanes read from a zero
    // vector. The shifts and masks constrain all byte values to `0..=63`.
    unsafe {
        let input_vec = v128_load(staged.as_ptr().cast());
        let zero_vec = u8x16_splat(0);
        let lanes = u8x16_shuffle::<2, 1, 0, 16, 5, 4, 3, 16, 8, 7, 6, 16, 11, 10, 9, 16>(
            input_vec, zero_vec,
        );

        let index0 = v128_and(u32x4_shr(lanes, 18), u32x4_splat(0x0000_003f));
        let index1 = v128_and(u32x4_shr(lanes, 4), u32x4_splat(0x0000_3f00));
        let index2 = v128_and(u32x4_shl(lanes, 10), u32x4_splat(0x003f_0000));
        let index3 = v128_and(u32x4_shl(lanes, 24), u32x4_splat(0x3f00_0000));
        let indices = v128_or(v128_or(index0, index1), v128_or(index2, index3));

        let encoded = encode_standard_family_indices_wasm::<A>(indices);
        v128_store(output.as_mut_ptr().cast(), encoded);
    }
    crate::wipe_bytes(&mut staged);
}

#[target_feature(enable = "simd128")]
unsafe fn encode_standard_family_indices_wasm<A>(indices: v128) -> v128
where
    A: Alphabet,
{
    let upper = u8x16_lt(indices, u8x16_splat(26));
    let lower = v128_and(
        u8x16_ge(indices, u8x16_splat(26)),
        u8x16_lt(indices, u8x16_splat(52)),
    );
    let digit = v128_and(
        u8x16_ge(indices, u8x16_splat(52)),
        u8x16_lt(indices, u8x16_splat(62)),
    );
    let plus = u8x16_eq(indices, u8x16_splat(62));
    let slash = u8x16_eq(indices, u8x16_splat(63));
    let plus_char = A::ENCODE[62];
    let slash_char = A::ENCODE[63];

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
    encoded = v128_bitselect(u8x16_splat(plus_char), encoded, plus);
    v128_bitselect(u8x16_splat(slash_char), encoded, slash)
}

struct AnchorMatchingCustom;

impl Alphabet for AnchorMatchingCustom {
    const ENCODE: [u8; 64] = *b"ACBDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

fn fill_pattern(output: &mut [u8], seed: u8) {
    let mut value = seed.wrapping_mul(19);
    for byte in output {
        *byte = value;
        value = value.wrapping_add(73);
    }
}

fn fill_indices_pattern(output: &mut [u8; 12], seed: u8) {
    let mut write = 0;
    for group in 0..4 {
        let i0 = seed.wrapping_add(group * 4) & 0x3f;
        let i1 = seed.wrapping_add(group * 4 + 1) & 0x3f;
        let i2 = seed.wrapping_add(group * 4 + 2) & 0x3f;
        let i3 = seed.wrapping_add(group * 4 + 3) & 0x3f;

        output[write] = (i0 << 2) | (i1 >> 4);
        output[write + 1] = (i1 << 4) | (i2 >> 2);
        output[write + 2] = (i2 << 6) | i3;
        write += 3;
    }
}

#[test]
fn wasm_simd128_encode_prototype_matches_scalar_when_available() {
    if !super::wasm_simd128_available() {
        println!("skipped: wasm simd128 prototype test requires target-feature=+simd128");
        return;
    }

    let mut input = [0; 12];
    for seed in 0..64u8 {
        fill_pattern(&mut input, seed);

        let mut wasm_standard = [0x55; 16];
        let mut scalar_standard = [0xaa; 16];
        // SAFETY: The candidate check above proves simd128 target-feature
        // availability for this test invocation.
        unsafe {
            encode_12_bytes_wasm_simd128::<Standard>(&input, &mut wasm_standard);
        }
        let scalar_result =
            Engine::<Standard, true>::new().encode_slice(&input, &mut scalar_standard);
        assert_eq!(scalar_result, Ok(wasm_standard.len()));
        assert_eq!(wasm_standard, scalar_standard);

        let mut wasm_url_safe = [0x55; 16];
        let mut scalar_url_safe = [0xaa; 16];
        // SAFETY: The candidate check above proves simd128 target-feature
        // availability for this test invocation.
        unsafe {
            encode_12_bytes_wasm_simd128::<UrlSafe>(&input, &mut wasm_url_safe);
        }
        let scalar_result =
            Engine::<UrlSafe, true>::new().encode_slice(&input, &mut scalar_url_safe);
        assert_eq!(scalar_result, Ok(wasm_url_safe.len()));
        assert_eq!(wasm_url_safe, scalar_url_safe);
    }

    for seed in 0..64u8 {
        fill_indices_pattern(&mut input, seed);

        let mut wasm_standard = [0x55; 16];
        let mut scalar_standard = [0xaa; 16];
        // SAFETY: The candidate check above proves simd128 target-feature
        // availability for this test invocation.
        unsafe {
            encode_12_bytes_wasm_simd128::<Standard>(&input, &mut wasm_standard);
        }
        let scalar_result =
            Engine::<Standard, true>::new().encode_slice(&input, &mut scalar_standard);
        assert_eq!(scalar_result, Ok(wasm_standard.len()));
        assert_eq!(wasm_standard, scalar_standard);

        let mut wasm_url_safe = [0x55; 16];
        let mut scalar_url_safe = [0xaa; 16];
        // SAFETY: The candidate check above proves simd128 target-feature
        // availability for this test invocation.
        unsafe {
            encode_12_bytes_wasm_simd128::<UrlSafe>(&input, &mut wasm_url_safe);
        }
        let scalar_result =
            Engine::<UrlSafe, true>::new().encode_slice(&input, &mut scalar_url_safe);
        assert_eq!(scalar_result, Ok(wasm_url_safe.len()));
        assert_eq!(wasm_url_safe, scalar_url_safe);
    }

    fill_indices_pattern(&mut input, 0);
    let mut wasm_custom = [0x55; 16];
    let mut scalar_custom = [0xaa; 16];
    // SAFETY: The candidate check above proves simd128 target-feature
    // availability for this test invocation. Custom alphabets intentionally
    // exercise the scalar fallback scaffold.
    unsafe {
        encode_12_bytes_wasm_simd128::<AnchorMatchingCustom>(&input, &mut wasm_custom);
    }
    let scalar_result =
        Engine::<AnchorMatchingCustom, true>::new().encode_slice(&input, &mut scalar_custom);
    assert_eq!(scalar_result, Ok(wasm_custom.len()));
    assert_eq!(wasm_custom, scalar_custom);
}
