#![allow(unsafe_code)]

#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    any(test, feature = "simd")
))]
mod direct;
#[cfg(any(test, all(feature = "simd", target_arch = "aarch64")))]
use crate::Alphabet;
#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    any(test, feature = "simd")
))]
use crate::Standard;
#[cfg(all(
    test,
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
use crate::encode_base64_value;
#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
use crate::{EncodeError, checked_encoded_len, scalar};

#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    any(test, feature = "simd")
))]
const NEON_DECODE_INPUT_BLOCK: usize = 16;
#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    any(test, feature = "simd")
))]
const NEON_DECODE_OUTPUT_BLOCK: usize = 12;

#[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
use core::arch::arm::{vdupq_n_u8, vst1q_u8};

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
pub(crate) fn neon_supports_alphabet<A>() -> bool
where
    A: Alphabet,
{
    is_standard_or_url_safe_family::<A>()
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
pub(crate) fn neon_supports_decode_alphabet<A>() -> bool
where
    A: Alphabet,
{
    neon_supports_alphabet::<A>()
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
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

    // SAFETY: Health-gated dispatch reaches this function only on
    // little-endian AArch64 where runtime, static, or unsafe-token evidence
    // proves NEON. Output sizing was preflighted above.
    let (read, write) = unsafe { encode_full_blocks_neon::<A>(input, output) };

    let tail_written = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail_written)
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
pub(crate) fn decode_slice_neon<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, crate::DecodeError>
where
    A: Alphabet,
{
    if input.len() < NEON_DECODE_INPUT_BLOCK || !neon_supports_decode_alphabet::<A>() {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    let required = scalar::validate_decode::<A, PAD>(input)?;
    if output.len() < required {
        return Err(crate::DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let simd_input_len = if input.last() == Some(&b'=') {
        input.len().saturating_sub(4)
    } else {
        input.len()
    };
    // SAFETY: Health-gated dispatch or the static token proves NEON. Scalar
    // validation and the output preflight prove exact block bounds. The final
    // padded quantum is excluded and remains on the scalar tail.
    let (read, write, classified) =
        unsafe { decode_full_blocks_neon::<A>(input, output, simd_input_len) };
    if !classified {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    let tail_written = scalar::decode_slice::<A, PAD>(&input[read..], &mut output[write..])
        .map_err(|error| error.with_index_offset(read))?;
    Ok(write + tail_written)
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub(crate) fn neon_available() -> bool {
    cfg!(target_arch = "aarch64") || cfg!(target_feature = "neon")
}

#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    any(test, feature = "simd")
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
            "eor v0.16b, v0.16b, v0.16b\neor v1.16b, v1.16b, v1.16b\neor v2.16b, v2.16b, v2.16b\neor v3.16b, v3.16b, v3.16b\neor v4.16b, v4.16b, v4.16b\neor v5.16b, v5.16b, v5.16b\neor v6.16b, v6.16b, v6.16b\neor v7.16b, v7.16b, v7.16b",
            "eor v8.16b, v8.16b, v8.16b\neor v9.16b, v9.16b, v9.16b\neor v10.16b, v10.16b, v10.16b\neor v11.16b, v11.16b, v11.16b\neor v12.16b, v12.16b, v12.16b\neor v13.16b, v13.16b, v13.16b\neor v14.16b, v14.16b, v14.16b\neor v15.16b, v15.16b, v15.16b",
            "eor v16.16b, v16.16b, v16.16b\neor v17.16b, v17.16b, v17.16b\neor v18.16b, v18.16b, v18.16b\neor v19.16b, v19.16b, v19.16b\neor v20.16b, v20.16b, v20.16b\neor v21.16b, v21.16b, v21.16b\neor v22.16b, v22.16b, v22.16b\neor v23.16b, v23.16b, v23.16b",
            "eor v24.16b, v24.16b, v24.16b\neor v25.16b, v25.16b, v25.16b\neor v26.16b, v26.16b, v26.16b\neor v27.16b, v27.16b, v27.16b\neor v28.16b, v28.16b, v28.16b\neor v29.16b, v29.16b, v29.16b\neor v30.16b, v30.16b, v30.16b\neor v31.16b, v31.16b, v31.16b",
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

/// Encodes one 12-byte block into 16 bytes through the NEON block encoder.
///
/// On little-endian `aarch64`, Standard and URL-safe alphabets use real NEON
/// fixed-block logic. Other alphabets, big-endian `AArch64`, and 32-bit
/// `arm+neon` builds use the scalar fallback scaffold.
///
/// # Safety
///
/// The caller must execute this function only when NEON is available on the
/// current CPU. NEON is mandatory on `aarch64`; `arm` builds must enable the
/// `neon` target feature. The input and output sizes are fixed by their array
/// types.
#[cfg(all(
    test,
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
pub(super) unsafe fn encode_12_bytes_neon<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    {
        if is_standard_or_url_safe_family::<A>() {
            // SAFETY: The caller has proven NEON availability. The helper uses
            // fixed input/output arrays and supports this alphabet family.
            unsafe {
                direct::encode_12_bytes::<A>(input, output);
                clear_neon_registers_after_vector_block!();
            }
            return;
        }
    }

    // Temporary 32-bit ARM/custom-alphabet scaffolding.
    #[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
    // SAFETY: `output` is a valid 16-byte mutable array and NEON availability
    // is guaranteed by this function's precondition.
    unsafe {
        let zeros = vdupq_n_u8(0);
        vst1q_u8(output.as_mut_ptr(), zeros);
    }

    scalar_encode_block::<A>(input, output);
}

/// Decodes one 16-byte Base64 block into at most 12 bytes through the NEON
/// block decoder.
///
/// This test-facing wrapper validates the block before entering the direct
/// NEON kernel. Production slice decode validates the complete input once and
/// then writes direct fixed blocks into the caller output.
///
/// # Safety
///
/// The caller must execute this function only when NEON is available on the
/// current CPU. The input and output sizes are fixed by their array types.
#[cfg(all(test, target_arch = "aarch64", target_endian = "little"))]
pub(crate) unsafe fn decode_16_bytes_neon<A, const PAD: bool>(
    input: &[u8; 16],
    output: &mut [u8; 12],
) -> Result<usize, crate::DecodeError>
where
    A: Alphabet,
{
    let written = crate::scalar::validate_decode::<A, PAD>(input)?;
    if written != NEON_DECODE_OUTPUT_BLOCK
        || !is_standard_or_url_safe_family::<A>()
        || input.contains(&b'=')
    {
        return crate::scalar::decode_slice::<A, PAD>(input, output);
    }

    // SAFETY: Scalar validation proves a canonical unpadded block; fixed
    // arrays prove exact load/store bounds and the caller proves NEON.
    let classified = unsafe {
        let classified = direct::decode_16_bytes::<A>(input, output);
        clear_neon_registers_after_vector_block!();
        classified
    };
    if !classified {
        return crate::scalar::decode_slice::<A, PAD>(input, output);
    }
    Ok(NEON_DECODE_OUTPUT_BLOCK)
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
unsafe fn encode_full_blocks_neon<A>(input: &[u8], output: &mut [u8]) -> (usize, usize)
where
    A: Alphabet,
{
    let mut read = 0;
    let mut write = 0;
    while read + 12 <= input.len() {
        // SAFETY: Loop guards and prior output preflight prove exact block
        // bounds. The caller carries the NEON availability contract.
        unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; 12]>());
            let encoded = &mut *(output.as_mut_ptr().add(write).cast::<[u8; 16]>());
            direct::encode_12_bytes::<A>(block, encoded);
        }
        read += 12;
        write += 16;
    }
    // SAFETY: The caller established NEON availability, and no vector value
    // produced by the completed block loop is needed after this point.
    unsafe { clear_neon_registers_after_vector_block!() };
    (read, write)
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
unsafe fn decode_full_blocks_neon<A>(
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
    while read + NEON_DECODE_INPUT_BLOCK <= simd_input_len {
        // SAFETY: Loop guards and prior validation/preflight prove exact block
        // bounds. The caller carries the NEON availability contract.
        let block_classified = unsafe {
            let block = &*(input
                .as_ptr()
                .add(read)
                .cast::<[u8; NEON_DECODE_INPUT_BLOCK]>());
            let decoded = &mut *(output
                .as_mut_ptr()
                .add(write)
                .cast::<[u8; NEON_DECODE_OUTPUT_BLOCK]>());
            direct::decode_16_bytes::<A>(block, decoded)
        };
        if !block_classified {
            classified = false;
            break;
        }
        read += NEON_DECODE_INPUT_BLOCK;
        write += NEON_DECODE_OUTPUT_BLOCK;
    }
    if read != 0 || !classified {
        // SAFETY: The caller established NEON availability, and both success
        // and classification-failure paths have finished using vector state.
        unsafe { clear_neon_registers_after_vector_block!() };
    }
    (read, write, classified)
}

#[cfg(all(
    test,
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
    target_endian = "little",
    any(test, feature = "simd")
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

#[cfg(all(test, target_arch = "aarch64", target_endian = "little"))]
pub(in crate::simd) fn test_direct_encode_12<A: Alphabet>(
    input: &[u8; 12],
    output: &mut [u8; 16],
) -> bool {
    if !neon_available() || !is_standard_or_url_safe_family::<A>() {
        return false;
    }
    // SAFETY: AArch64 guarantees NEON, the alphabet was admitted, and fixed
    // arrays prove the direct block bounds.
    unsafe {
        direct::encode_12_bytes::<A>(input, output);
        clear_neon_registers_after_vector_block!();
    }
    true
}

#[cfg(all(test, target_arch = "aarch64", target_endian = "little"))]
pub(in crate::simd) fn test_direct_decode_16<A: Alphabet>(
    input: &[u8; 16],
    output: &mut [u8; 12],
) -> bool {
    if !neon_available() || !is_standard_or_url_safe_family::<A>() {
        return false;
    }
    // SAFETY: AArch64 guarantees NEON, the alphabet was admitted, and fixed
    // arrays prove the direct block bounds.
    unsafe {
        let classified = direct::decode_16_bytes::<A>(input, output);
        clear_neon_registers_after_vector_block!();
        classified
    }
}
