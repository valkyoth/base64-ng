//! Encode backend dispatch boundary.
//!
//! This module is the single integration point between public encode APIs and
//! the implementation that performs encoding. AVX-512 VBMI, AVX2,
//! SSSE3/SSE4.1, and little-endian `AArch64` NEON encode dispatch is admitted
//! only for std builds and Standard/URL-safe alphabet families; unsupported
//! alphabets and targets still fall back to scalar. In-place encode uses
//! stack staging before entering admitted encode backends so output writes do
//! not overwrite unread input bytes.

use crate::{Alphabet, EncodeError, scalar, scalar_encode_in_place};
#[cfg(any(
    all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ),
    all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ),
    all(feature = "simd", target_arch = "wasm32")
))]
use crate::{checked_encoded_len, wipe_bytes};

const MIN_SIMD_ENCODE_BLOCK: usize = 12;
#[cfg(any(
    all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ),
    all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ),
    all(feature = "simd", target_arch = "wasm32")
))]
const IN_PLACE_INPUT_CHUNK: usize = 768;
#[cfg(any(
    all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ),
    all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ),
    all(feature = "simd", target_arch = "wasm32")
))]
const IN_PLACE_OUTPUT_CHUNK: usize = 1024;

/// Encode backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EncodeBackend {
    /// The audited scalar implementation.
    Scalar,
    /// std `x86`/`x86_64` AVX-512 VBMI fixed-block encode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Avx512Vbmi,
    /// std `x86`/`x86_64` AVX2 fixed-block encode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Avx2,
    /// std `x86`/`x86_64` SSSE3/SSE4.1 fixed-block encode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Ssse3Sse41,
    /// little-endian std `aarch64` NEON fixed-block encode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ))]
    Neon,
    /// wasm32 `simd128` fixed-block encode.
    #[cfg(all(feature = "simd", target_arch = "wasm32"))]
    WasmSimd128,
}

/// Returns the encode backend selected for this build and target.
#[must_use]
pub(crate) fn active_encode_backend() -> EncodeBackend {
    #[cfg(feature = "simd")]
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => {}
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx512Vbmi => return EncodeBackend::Avx512Vbmi,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx2 => return EncodeBackend::Avx2,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Ssse3Sse41 => return EncodeBackend::Ssse3Sse41,
        #[cfg(all(feature = "std", target_arch = "aarch64", target_endian = "little"))]
        crate::simd::ActiveBackend::Neon => return EncodeBackend::Neon,
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        crate::simd::ActiveBackend::WasmSimd128 => return EncodeBackend::WasmSimd128,
    }

    EncodeBackend::Scalar
}

/// Encodes `input` into `output` through the admitted encode backend.
pub(crate) fn encode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    if input.len() < MIN_SIMD_ENCODE_BLOCK {
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    match active_encode_backend() {
        EncodeBackend::Scalar => scalar::encode_slice::<A, PAD>(input, output),
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Avx512Vbmi => {
            if input.len() >= 48 && crate::simd::avx512_supports_alphabet::<A>() {
                crate::simd::encode_slice_avx512::<A, PAD>(input, output)
            } else {
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Avx2 => {
            if input.len() >= 24 && crate::simd::avx2_supports_alphabet::<A>() {
                crate::simd::encode_slice_avx2::<A, PAD>(input, output)
            } else {
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Ssse3Sse41 => {
            if input.len() >= 12 && crate::simd::ssse3_sse41_supports_alphabet::<A>() {
                crate::simd::encode_slice_ssse3_sse41::<A, PAD>(input, output)
            } else {
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            target_arch = "aarch64",
            target_endian = "little"
        ))]
        EncodeBackend::Neon => {
            if input.len() >= 12 && crate::simd::neon_supports_alphabet::<A>() {
                crate::simd::encode_slice_neon::<A, PAD>(input, output)
            } else {
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        EncodeBackend::WasmSimd128 => {
            if input.len() >= 12 && crate::simd::wasm_simd128_supports_alphabet::<A>() {
                crate::simd::encode_slice_wasm_simd128::<A, PAD>(input, output)
            } else {
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
    }
}

/// Encodes `buffer[..input_len]` in place through the admitted encode backend.
pub(crate) fn encode_in_place<A, const PAD: bool>(
    buffer: &mut [u8],
    input_len: usize,
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    match active_encode_backend() {
        EncodeBackend::Scalar => {
            scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Avx512Vbmi => {
            if input_len >= 48 && crate::simd::avx512_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Avx2 => {
            if input_len >= 24 && crate::simd::avx2_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Ssse3Sse41 => {
            if input_len >= 12 && crate::simd::ssse3_sse41_supports_alphabet::<A>() {
                encode_in_place_staged::<A, PAD>(buffer, input_len)
            } else {
                scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
            }
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            target_arch = "aarch64",
            target_endian = "little"
        ))]
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
    }
}

#[cfg(any(
    all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ),
    all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ),
    all(feature = "simd", target_arch = "wasm32")
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

    let mut input_scratch = [0u8; IN_PLACE_INPUT_CHUNK];
    let mut output_scratch = [0u8; IN_PLACE_OUTPUT_CHUNK];
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
        let written = match encode_slice::<A, PAD>(
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
    all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ),
    all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ),
    all(feature = "simd", target_arch = "wasm32")
))]
fn in_place_chunk_start(remaining: usize) -> Result<usize, EncodeError> {
    if remaining <= IN_PLACE_INPUT_CHUNK {
        Ok(0)
    } else {
        round_up_to_multiple_of_three(remaining - IN_PLACE_INPUT_CHUNK)
    }
}

#[cfg(any(
    all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ),
    all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ),
    all(feature = "simd", target_arch = "wasm32")
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
