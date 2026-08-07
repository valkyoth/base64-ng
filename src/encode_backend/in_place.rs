//! In-place encode dispatch and bounded staging.

use super::{EncodeBackend, active_encode_backend_for_input};
use crate::{Alphabet, EncodeError, scalar_encode_in_place};

#[cfg(any(
    all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
    all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
    all(feature = "simd", target_arch = "wasm32"),
    all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    )
))]
use crate::{checked_encoded_len, wipe_bytes};

#[cfg(any(
    all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
    all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
    all(feature = "simd", target_arch = "wasm32"),
    all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    )
))]
const INPUT_CHUNK: usize = 768;
#[cfg(any(
    all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
    all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
    all(feature = "simd", target_arch = "wasm32"),
    all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    )
))]
const OUTPUT_CHUNK: usize = 1024;

/// Encodes `buffer[..input_len]` in place through the admitted encode backend.
pub(crate) fn encode_in_place<A, const PAD: bool>(
    buffer: &mut [u8],
    input_len: usize,
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    match active_encode_backend_for_input(input_len) {
        EncodeBackend::Scalar => {
            scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Avx512Vbmi => {
            if input_len >= 48 && crate::simd::avx512_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Avx2 => {
            if input_len >= 24 && crate::simd::avx2_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Ssse3Sse41 => {
            if input_len >= 12 && crate::simd::ssse3_sse41_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        EncodeBackend::Neon => {
            if input_len >= 12 && crate::simd::neon_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        EncodeBackend::WasmSimd128 => {
            if input_len >= 12 && crate::simd::wasm_simd128_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(
            feature = "std",
            feature = "simd",
            target_arch = "riscv64",
            target_os = "linux"
        ))]
        EncodeBackend::Rvv => {
            if input_len >= 12 && crate::simd::rvv_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
    }
}

#[cfg(any(
    all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
    all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
    all(feature = "simd", target_arch = "wasm32"),
    all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    )
))]
fn encode_in_place_staged<A, const PAD: bool>(
    buffer: &mut [u8],
    input_len: usize,
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    if input_len > buffer.len() {
        return Err(EncodeError::InputTooLarge {
            input_len,
            buffer_len: buffer.len(),
        });
    }

    let required = checked_encoded_len(input_len, PAD).ok_or(EncodeError::LengthOverflow)?;
    if buffer.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: buffer.len(),
        });
    }

    let mut input_scratch = [0u8; INPUT_CHUNK];
    let mut output_scratch = [0u8; OUTPUT_CHUNK];
    let mut remaining = input_len;
    let mut output_end = required;

    while remaining != 0 {
        let chunk_start = in_place_chunk_start(remaining)?;
        let chunk_len = remaining - chunk_start;
        let output_start =
            checked_encoded_len(chunk_start, PAD).ok_or(EncodeError::LengthOverflow)?;
        let expected_output_len = output_end - output_start;
        if chunk_len > input_scratch.len() || expected_output_len > output_scratch.len() {
            return Err(EncodeError::LengthOverflow);
        }

        input_scratch[..chunk_len].copy_from_slice(&buffer[chunk_start..remaining]);
        let written = match super::encode_slice::<A, PAD>(
            &input_scratch[..chunk_len],
            &mut output_scratch[..expected_output_len],
        ) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut input_scratch[..chunk_len]);
                wipe_bytes(&mut output_scratch[..expected_output_len]);
                return Err(err);
            }
        };

        debug_assert_eq!(
            written, expected_output_len,
            "encode_in_place_staged chunk length mismatch"
        );
        if written != expected_output_len {
            wipe_bytes(&mut input_scratch[..chunk_len]);
            wipe_bytes(&mut output_scratch[..expected_output_len]);
            return Err(EncodeError::LengthOverflow);
        }

        buffer[output_start..output_end].copy_from_slice(&output_scratch[..written]);
        wipe_bytes(&mut input_scratch[..chunk_len]);
        wipe_bytes(&mut output_scratch[..written]);

        remaining = chunk_start;
        output_end = output_start;
    }

    Ok(required)
}

#[cfg(any(
    all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
    all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
    all(feature = "simd", target_arch = "wasm32"),
    all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    )
))]
fn in_place_chunk_start(remaining: usize) -> Result<usize, EncodeError> {
    if remaining <= INPUT_CHUNK {
        Ok(0)
    } else {
        round_up_to_multiple_of_three(remaining - INPUT_CHUNK)
    }
}

#[cfg(any(
    all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
    all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
    all(feature = "simd", target_arch = "wasm32"),
    all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    )
))]
fn round_up_to_multiple_of_three(value: usize) -> Result<usize, EncodeError> {
    let remainder = value % 3;
    if remainder == 0 {
        Ok(value)
    } else {
        value
            .checked_add(3 - remainder)
            .ok_or(EncodeError::LengthOverflow)
    }
}
