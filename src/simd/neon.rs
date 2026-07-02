#![allow(unsafe_code)]

#[cfg(any(test, all(feature = "std", target_arch = "aarch64")))]
use crate::Alphabet;
#[cfg(all(
    target_arch = "aarch64",
    any(test, all(feature = "std", feature = "simd"))
))]
use crate::Standard;
#[cfg(all(
    any(test, all(feature = "std", feature = "simd", target_arch = "aarch64")),
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
use crate::encode_base64_value;
#[cfg(all(feature = "std", feature = "simd", target_arch = "aarch64"))]
use crate::{EncodeError, checked_encoded_len, scalar};

#[cfg(all(
    target_arch = "aarch64",
    any(test, all(feature = "std", feature = "simd"))
))]
use core::arch::aarch64::{
    uint8x16_t, uint32x4_t, vaddq_u8, vandq_u8, vandq_u32, vbslq_u8, vceqq_u8, vcgeq_u8, vcltq_u8,
    vdupq_n_u8, vdupq_n_u32, vld1q_u8, vorrq_u32, vqtbl1q_u8, vreinterpretq_u8_u32,
    vreinterpretq_u32_u8, vshlq_n_u32, vshrq_n_u32, vst1q_u8, vsubq_u8,
};
#[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
use core::arch::arm::{uint8x16_t, vdupq_n_u8, vst1q_u8};

#[cfg(all(feature = "std", target_arch = "aarch64"))]
pub(crate) fn neon_supports_alphabet<A>() -> bool
where
    A: Alphabet,
{
    is_standard_or_url_safe_family::<A>()
}

#[cfg(all(feature = "std", feature = "simd", target_arch = "aarch64"))]
pub(crate) fn encode_slice_neon<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    if input.len() < 12 {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    if !neon_supports_alphabet::<A>() {
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
    while read + 12 <= input.len() {
        // SAFETY: Runtime dispatch reaches this function only on std AArch64
        // where NEON is part of the target contract. The fixed arrays satisfy
        // the block encoder's size preconditions.
        unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; 12]>());
            let encoded = &mut *(output.as_mut_ptr().add(write).cast::<[u8; 16]>());
            encode_12_bytes_neon::<A>(block, encoded);
        }
        read += 12;
        write += 16;
    }

    let tail_written = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail_written)
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub(crate) fn neon_available() -> bool {
    cfg!(target_arch = "aarch64") || cfg!(target_feature = "neon")
}

/// Encodes one 12-byte block into 16 bytes through the NEON block encoder.
///
/// On `aarch64`, Standard and URL-safe alphabets use real NEON fixed-block
/// logic. Other alphabets and 32-bit `arm+neon` builds use the scalar fallback
/// scaffold.
///
/// Admission note: a real NEON implementation must explicitly clear every
/// vector register that carries caller data before returning, document the
/// exact cleanup sequence in `docs/UNSAFE.md`, and include generated-assembly
/// evidence. The `AArch64` prototype clears its used NEON registers before
/// return as best-effort register-retention reduction.
///
/// # Safety
///
/// The caller must execute this function only when NEON is available on the
/// current CPU. NEON is mandatory on `aarch64`; `arm` builds must enable the
/// `neon` target feature. The input and output sizes are fixed by their array
/// types.
#[cfg(all(
    any(test, all(feature = "std", feature = "simd")),
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
pub(super) unsafe fn encode_12_bytes_neon<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    #[cfg(target_arch = "aarch64")]
    {
        if is_standard_or_url_safe_family::<A>() {
            // SAFETY: The caller has proven NEON availability. The helper uses
            // fixed input/output arrays and supports this alphabet family.
            unsafe {
                encode_12_bytes_neon_aarch64_standard_family::<A>(input, output);
            }
            return;
        }
    }

    // Temporary 32-bit ARM/custom-alphabet scaffolding.
    #[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
    // SAFETY: `output` is a valid 16-byte mutable array and NEON availability
    // is guaranteed by this function's precondition.
    unsafe {
        let zeros: uint8x16_t = vdupq_n_u8(0);
        vst1q_u8(output.as_mut_ptr(), zeros);
    }

    scalar_encode_block::<A>(input, output);
}

#[cfg(all(
    target_arch = "aarch64",
    any(test, all(feature = "std", feature = "simd"))
))]
macro_rules! clear_neon_registers_after_vector_block {
    () => {{
        // SAFETY: This cleanup is expanded directly inside the block encoder
        // or decoder after it stores local output. There is no separate helper
        // frame whose ABI save/restore can undo `v8..v15` clearing. The
        // explicit outputs tell the compiler every AArch64 vector register is
        // clobbered while the assembly clears it. This is retention reduction
        // for SIMD evidence, not a formal microarchitectural proof.
        core::arch::asm!(
            "eor v0.16b, v0.16b, v0.16b",
            "eor v1.16b, v1.16b, v1.16b",
            "eor v2.16b, v2.16b, v2.16b",
            "eor v3.16b, v3.16b, v3.16b",
            "eor v4.16b, v4.16b, v4.16b",
            "eor v5.16b, v5.16b, v5.16b",
            "eor v6.16b, v6.16b, v6.16b",
            "eor v7.16b, v7.16b, v7.16b",
            "eor v8.16b, v8.16b, v8.16b",
            "eor v9.16b, v9.16b, v9.16b",
            "eor v10.16b, v10.16b, v10.16b",
            "eor v11.16b, v11.16b, v11.16b",
            "eor v12.16b, v12.16b, v12.16b",
            "eor v13.16b, v13.16b, v13.16b",
            "eor v14.16b, v14.16b, v14.16b",
            "eor v15.16b, v15.16b, v15.16b",
            "eor v16.16b, v16.16b, v16.16b",
            "eor v17.16b, v17.16b, v17.16b",
            "eor v18.16b, v18.16b, v18.16b",
            "eor v19.16b, v19.16b, v19.16b",
            "eor v20.16b, v20.16b, v20.16b",
            "eor v21.16b, v21.16b, v21.16b",
            "eor v22.16b, v22.16b, v22.16b",
            "eor v23.16b, v23.16b, v23.16b",
            "eor v24.16b, v24.16b, v24.16b",
            "eor v25.16b, v25.16b, v25.16b",
            "eor v26.16b, v26.16b, v26.16b",
            "eor v27.16b, v27.16b, v27.16b",
            "eor v28.16b, v28.16b, v28.16b",
            "eor v29.16b, v29.16b, v29.16b",
            "eor v30.16b, v30.16b, v30.16b",
            "eor v31.16b, v31.16b, v31.16b",
            out("v0") _,
            out("v1") _,
            out("v2") _,
            out("v3") _,
            out("v4") _,
            out("v5") _,
            out("v6") _,
            out("v7") _,
            out("v8") _,
            out("v9") _,
            out("v10") _,
            out("v11") _,
            out("v12") _,
            out("v13") _,
            out("v14") _,
            out("v15") _,
            out("v16") _,
            out("v17") _,
            out("v18") _,
            out("v19") _,
            out("v20") _,
            out("v21") _,
            out("v22") _,
            out("v23") _,
            out("v24") _,
            out("v25") _,
            out("v26") _,
            out("v27") _,
            out("v28") _,
            out("v29") _,
            out("v30") _,
            out("v31") _,
            options(nostack, preserves_flags)
        );
    }};
}

/// Decodes one 16-byte Base64 block into at most 12 bytes through the NEON
/// block decoder.
///
/// This is non-dispatchable evidence for the `1.3.0` decode admission line.
/// It validates with the scalar decoder before copying any bytes into the
/// caller-visible output, so malformed inputs cannot expose prototype output.
///
/// # Safety
///
/// The caller must execute this function only when NEON is available on the
/// current CPU. The input and output sizes are fixed by their array types.
#[cfg(all(test, target_arch = "aarch64"))]
pub(super) unsafe fn decode_16_bytes_neon<A, const PAD: bool>(
    input: &[u8; 16],
    output: &mut [u8; 12],
) -> Result<usize, crate::DecodeError>
where
    A: Alphabet,
{
    let mut scalar_output = [0; 12];
    let written = match crate::scalar::decode_slice::<A, PAD>(input, &mut scalar_output) {
        Ok(written) => written,
        Err(error) => {
            crate::wipe_bytes(&mut scalar_output);
            return Err(error);
        }
    };

    if !is_standard_or_url_safe_family::<A>() {
        output[..written].copy_from_slice(&scalar_output[..written]);
        crate::wipe_bytes(&mut scalar_output);
        return Ok(written);
    }

    let mut values = [0; 16];
    fill_decode_values::<A, 16>(input, &mut values);
    let mut packed = [0; 16];
    let compact = [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14, 255, 255, 255, 255];

    // SAFETY: Fixed arrays back every load and store, scalar validation above
    // proves the 16-byte block is canonical for the selected padding policy,
    // and the NEON target-feature contract enables the vector packing and
    // cleanup instructions used here.
    unsafe {
        let values_vec = vld1q_u8(values.as_ptr());
        let lanes: uint32x4_t = vreinterpretq_u32_u8(values_vec);

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
        let compact_vec = vld1q_u8(compact.as_ptr());
        let decoded = vqtbl1q_u8(lane_bytes, compact_vec);
        vst1q_u8(packed.as_mut_ptr(), decoded);
        clear_neon_registers_after_vector_block!();
    }

    crate::wipe_bytes(&mut values);
    copy_verified_decode_output(&mut packed, &mut scalar_output, output, written)?;
    Ok(written)
}

#[cfg(all(
    any(test, all(feature = "std", feature = "simd", target_arch = "aarch64")),
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
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

#[cfg(all(
    target_arch = "aarch64",
    any(test, all(feature = "std", feature = "simd"))
))]
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

#[cfg(all(
    target_arch = "aarch64",
    any(test, all(feature = "std", feature = "simd"))
))]
#[target_feature(enable = "neon")]
unsafe fn encode_12_bytes_neon_aarch64_standard_family<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    let mut staged = [
        input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        input[9], input[10], input[11], 0, 0, 0, 0,
    ];
    let shuffle_mask = [2, 1, 0, 255, 5, 4, 3, 255, 8, 7, 6, 255, 11, 10, 9, 255];

    // SAFETY: Fixed arrays back every unaligned 128-bit load/store, the
    // target-feature contract enables NEON, shuffle zero lanes read only
    // staged zeros, and indices are masked to `0..=63`.
    unsafe {
        let input_vec = vld1q_u8(staged.as_ptr());
        let shuffle = vld1q_u8(shuffle_mask.as_ptr());
        let lanes = vqtbl1q_u8(input_vec, shuffle);
        let lane_words: uint32x4_t = vreinterpretq_u32_u8(lanes);

        let index0 = vandq_u32(vshrq_n_u32(lane_words, 18), vdupq_n_u32(0x0000_003f));
        let index1 = vandq_u32(vshrq_n_u32(lane_words, 4), vdupq_n_u32(0x0000_3f00));
        let index2 = vandq_u32(vshlq_n_u32(lane_words, 10), vdupq_n_u32(0x003f_0000));
        let index3 = vandq_u32(vshlq_n_u32(lane_words, 24), vdupq_n_u32(0x3f00_0000));
        let indices = vreinterpretq_u8_u32(vorrq_u32(
            vorrq_u32(index0, index1),
            vorrq_u32(index2, index3),
        ));

        let encoded = encode_standard_family_indices_neon::<A>(indices);
        vst1q_u8(output.as_mut_ptr(), encoded);
        clear_neon_registers_after_vector_block!();
    }
    crate::wipe_bytes(&mut staged);
}

#[cfg(all(
    target_arch = "aarch64",
    any(test, all(feature = "std", feature = "simd"))
))]
#[target_feature(enable = "neon")]
unsafe fn encode_standard_family_indices_neon<A>(indices: uint8x16_t) -> uint8x16_t
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
    let plus = vceqq_u8(indices, vdupq_n_u8(62));
    let slash = vceqq_u8(indices, vdupq_n_u8(63));
    let plus_char = A::ENCODE[62];
    let slash_char = A::ENCODE[63];

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
    encoded = vbslq_u8(plus, vdupq_n_u8(plus_char), encoded);
    vbslq_u8(slash, vdupq_n_u8(slash_char), encoded)
}

#[cfg(all(target_arch = "aarch64", test))]
fn copy_verified_decode_output<const PACKED: usize, const SCALAR: usize>(
    packed: &mut [u8; PACKED],
    scalar_output: &mut [u8; SCALAR],
    output: &mut [u8],
    written: usize,
) -> Result<(), crate::DecodeError> {
    if packed[..written] != scalar_output[..written] {
        crate::wipe_bytes(packed);
        crate::wipe_bytes(scalar_output);
        return Err(crate::DecodeError::InvalidInput);
    }

    output[..written].copy_from_slice(&packed[..written]);
    crate::wipe_bytes(packed);
    crate::wipe_bytes(scalar_output);
    Ok(())
}

#[cfg(all(target_arch = "aarch64", test))]
fn fill_decode_values<A, const N: usize>(input: &[u8; N], values: &mut [u8; N])
where
    A: Alphabet,
{
    let mut index = 0;
    while index < input.len() {
        values[index] = match input[index] {
            b'=' => 0,
            byte => {
                if let Some(value) = A::decode(byte) {
                    value
                } else {
                    debug_assert!(false, "fill_decode_values called on unvalidated input");
                    0
                }
            }
        };
        index += 1;
    }
}
