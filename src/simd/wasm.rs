mod direct;

use crate::{Alphabet, EncodeError, Standard, checked_encoded_len, scalar};

const ENCODE_INPUT_BLOCK: usize = 12;
const ENCODE_OUTPUT_BLOCK: usize = 16;
const DECODE_INPUT_BLOCK: usize = 16;
const DECODE_OUTPUT_BLOCK: usize = 12;

pub(crate) fn wasm_simd128_supports_alphabet<A: Alphabet>() -> bool {
    is_standard_or_url_safe_family::<A>()
}

pub(crate) fn wasm_simd128_supports_decode_alphabet<A: Alphabet>() -> bool {
    wasm_simd128_supports_alphabet::<A>() && super::decode_matches_encode_table::<A>()
}

pub(crate) fn wasm_simd128_decode_available() -> bool {
    super::wasm_simd128_available()
}

pub(crate) fn encode_slice_wasm_simd128<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    if input.len() < ENCODE_INPUT_BLOCK || !wasm_simd128_supports_alphabet::<A>() {
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
    while read + ENCODE_INPUT_BLOCK <= input.len() {
        // SAFETY: The loop guard and output preflight prove exact fixed-block
        // bounds. This artifact was compiled with `target-feature=+simd128`.
        unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; ENCODE_INPUT_BLOCK]>());
            let encoded = &mut *(output
                .as_mut_ptr()
                .add(write)
                .cast::<[u8; ENCODE_OUTPUT_BLOCK]>());
            direct::encode_12_bytes::<A>(block, encoded);
        }
        read += ENCODE_INPUT_BLOCK;
        write += ENCODE_OUTPUT_BLOCK;
    }

    let tail_written = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail_written)
}

pub(crate) fn decode_slice_wasm_simd128<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, crate::DecodeError> {
    if input.len() < DECODE_INPUT_BLOCK || !wasm_simd128_supports_decode_alphabet::<A>() {
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    let required = scalar::validate_decode::<A, PAD>(input)?;
    if output.len() < required {
        return Err(crate::DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    // Padding and canonical tail-bit handling stay in the scalar tail. A
    // padded final quantum is therefore never passed to the direct kernel.
    let simd_input_len = if PAD && input.last() == Some(&b'=') {
        input.len().saturating_sub(4)
    } else {
        input.len()
    };
    let (read, write, classified) = unsafe {
        // SAFETY: Whole-input scalar validation and output preflight completed
        // before the fixed-block loop. The artifact carries simd128 support.
        decode_full_blocks::<A>(input, output, simd_input_len)
    };
    if !classified {
        // The direct classifier must agree with scalar validation. Retrying
        // the whole output through scalar preserves correctness if a compiler
        // or runtime ever violates that internal invariant.
        return scalar::decode_slice::<A, PAD>(input, output);
    }

    let tail_written = scalar::decode_slice::<A, PAD>(&input[read..], &mut output[write..])
        .map_err(|error| error.with_index_offset(read))?;
    Ok(write + tail_written)
}

#[target_feature(enable = "simd128")]
unsafe fn decode_full_blocks<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
    simd_input_len: usize,
) -> (usize, usize, bool) {
    let mut read = 0;
    let mut write = 0;
    while read + DECODE_INPUT_BLOCK <= simd_input_len {
        // SAFETY: The loop guard, completed scalar validation, and output
        // preflight prove both exact blocks. The caller carries simd128.
        let classified = unsafe {
            let block = &*(input.as_ptr().add(read).cast::<[u8; DECODE_INPUT_BLOCK]>());
            let decoded = &mut *(output
                .as_mut_ptr()
                .add(write)
                .cast::<[u8; DECODE_OUTPUT_BLOCK]>());
            direct::decode_16_bytes::<A>(block, decoded)
        };
        if !classified {
            return (read, write, false);
        }
        read += DECODE_INPUT_BLOCK;
        write += DECODE_OUTPUT_BLOCK;
    }
    (read, write, true)
}

fn is_standard_or_url_safe_family<A: Alphabet>() -> bool {
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
