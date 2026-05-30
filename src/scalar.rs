//! Scalar encoding and strict decoding implementation.

use crate::{
    Alphabet, DecodeError, EncodeError, checked_encoded_len, decoded_len_padded,
    decoded_len_unpadded, encode_base64_value_runtime,
};

pub(crate) fn encode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    #[cfg(feature = "simd")]
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => {}
    }

    scalar_encode_slice::<A, PAD>(input, output)
}

pub(crate) fn decode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    #[cfg(feature = "simd")]
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => {}
    }

    scalar_decode_slice::<A, PAD>(input, output)
}

#[cfg(test)]
pub(crate) fn scalar_reference_encode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    scalar_encode_slice::<A, PAD>(input, output)
}

#[cfg(test)]
pub(crate) fn scalar_reference_decode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    scalar_decode_slice::<A, PAD>(input, output)
}

fn scalar_encode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
    if output.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + 3 <= input.len() {
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];

        output[write] = encode_base64_value_runtime::<A>(b0 >> 2);
        output[write + 1] = encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
        output[write + 2] = encode_base64_value_runtime::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
        output[write + 3] = encode_base64_value_runtime::<A>(b2 & 0b0011_1111);

        read += 3;
        write += 4;
    }

    match input.len() - read {
        0 => {}
        1 => {
            let b0 = input[read];
            output[write] = encode_base64_value_runtime::<A>(b0 >> 2);
            output[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
            write += 2;
            if PAD {
                output[write] = b'=';
                output[write + 1] = b'=';
                write += 2;
            }
        }
        2 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            output[write] = encode_base64_value_runtime::<A>(b0 >> 2);
            output[write + 1] =
                encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            output[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
            write += 3;
            if PAD {
                output[write] = b'=';
                write += 1;
            }
        }
        _ => unreachable!(),
    }

    Ok(write)
}

fn scalar_decode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        decode_padded::<A>(input, output)
    } else {
        decode_unpadded::<A>(input, output)
    }
}

pub(crate) fn decode_padded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let required = decoded_len_padded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read < input.len() {
        let chunk = read_quad(input, read)?;
        let available = output.len();
        let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
            required: write,
            available,
        })?;
        let written = decode_chunk::<A, true>(chunk, output_tail)
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
        if written < 3 && read != input.len() {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }
    Ok(write)
}

pub(crate) fn validate_decode<A: Alphabet, const PAD: bool>(
    input: &[u8],
) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        validate_padded::<A>(input)
    } else {
        validate_unpadded::<A>(input)
    }
}

fn validate_padded<A: Alphabet>(input: &[u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let required = decoded_len_padded(input)?;

    let mut read = 0;
    while read < input.len() {
        let chunk = read_quad(input, read)?;
        let written =
            validate_chunk::<A, true>(chunk).map_err(|err| err.with_index_offset(read))?;
        read += 4;
        if written < 3 && read != input.len() {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }

    Ok(required)
}

fn validate_unpadded<A: Alphabet>(input: &[u8]) -> Result<usize, DecodeError> {
    let required = decoded_len_unpadded(input)?;

    let mut read = 0;
    while read + 4 <= input.len() {
        let chunk = read_quad(input, read)?;
        validate_chunk::<A, false>(chunk).map_err(|err| err.with_index_offset(read))?;
        read += 4;
    }
    validate_tail_unpadded::<A>(&input[read..]).map_err(|err| err.with_index_offset(read))?;

    Ok(required)
}

pub(crate) fn decode_unpadded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    let required = decoded_len_unpadded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + 4 <= input.len() {
        let chunk = read_quad(input, read)?;
        let available = output.len();
        let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
            required: write,
            available,
        })?;
        let written = decode_chunk::<A, false>(chunk, output_tail)
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
    }
    decode_tail_unpadded::<A>(&input[read..], &mut output[write..])
        .map_err(|err| err.with_index_offset(read))
        .map(|n| write + n)
}

pub(crate) fn read_quad(input: &[u8], offset: usize) -> Result<[u8; 4], DecodeError> {
    let end = offset.checked_add(4).ok_or(DecodeError::InvalidLength)?;
    match input.get(offset..end) {
        Some([b0, b1, b2, b3]) => Ok([*b0, *b1, *b2, *b3]),
        _ => Err(DecodeError::InvalidLength),
    }
}

fn first_padding_index_unchecked(input: [u8; 4]) -> usize {
    let [b0, b1, b2, b3] = input;
    if b0 == b'=' {
        0
    } else if b1 == b'=' {
        1
    } else if b2 == b'=' {
        2
    } else if b3 == b'=' {
        3
    } else {
        debug_assert!(
            false,
            "first_padding_index_unchecked called with no padding"
        );
        4
    }
}

pub(crate) fn validate_chunk<A: Alphabet, const PAD: bool>(
    input: [u8; 4],
) -> Result<usize, DecodeError> {
    let [b0, b1, b2, b3] = input;
    let _v0 = decode_byte::<A>(b0, 0)?;
    let v1 = decode_byte::<A>(b1, 1)?;

    match (b2, b3) {
        (b'=', b'=') if PAD => {
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding { index: 2 }),
        (_, b'=') if PAD => {
            let v2 = decode_byte::<A>(b2, 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            Ok(2)
        }
        (b'=', _) | (_, b'=') => Err(DecodeError::InvalidPadding {
            index: first_padding_index_unchecked(input),
        }),
        _ => {
            decode_byte::<A>(b2, 2)?;
            decode_byte::<A>(b3, 3)?;
            Ok(3)
        }
    }
}

pub(crate) fn decode_chunk<A: Alphabet, const PAD: bool>(
    input: [u8; 4],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    let [b0, b1, b2, b3] = input;
    let v0 = decode_byte::<A>(b0, 0)?;
    let v1 = decode_byte::<A>(b1, 1)?;

    match (b2, b3) {
        (b'=', b'=') if PAD => {
            if output.is_empty() {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            }
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding { index: 2 }),
        (_, b'=') if PAD => {
            if output.len() < 2 {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(b2, 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        (b'=', _) | (_, b'=') => Err(DecodeError::InvalidPadding {
            index: first_padding_index_unchecked(input),
        }),
        _ => {
            if output.len() < 3 {
                return Err(DecodeError::OutputTooSmall {
                    required: 3,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(b2, 2)?;
            let v3 = decode_byte::<A>(b3, 3)?;
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            output[2] = (v2 << 6) | v3;
            Ok(3)
        }
    }
}

pub(crate) fn validate_tail_unpadded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    match input {
        [] => Ok(()),
        [b0, b1] => {
            decode_byte::<A>(*b0, 0)?;
            let v1 = decode_byte::<A>(*b1, 1)?;
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            Ok(())
        }
        [b0, b1, b2] => {
            decode_byte::<A>(*b0, 0)?;
            decode_byte::<A>(*b1, 1)?;
            let v2 = decode_byte::<A>(*b2, 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            Ok(())
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

pub(crate) fn decode_tail_unpadded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    match input {
        [] => Ok(0),
        [b0, b1] => {
            let Some(out0) = output.first_mut() else {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            };
            let v0 = decode_byte::<A>(*b0, 0)?;
            let v1 = decode_byte::<A>(*b1, 1)?;
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            *out0 = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        [b0, b1, b2] => {
            let available = output.len();
            let Some([out0, out1]) = output.get_mut(..2) else {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available,
                });
            };
            let v0 = decode_byte::<A>(*b0, 0)?;
            let v1 = decode_byte::<A>(*b1, 1)?;
            let v2 = decode_byte::<A>(*b2, 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            *out0 = (v0 << 2) | (v1 >> 4);
            *out1 = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

pub(crate) fn decode_byte<A: Alphabet>(byte: u8, index: usize) -> Result<u8, DecodeError> {
    A::decode(byte).ok_or(DecodeError::InvalidByte { index, byte })
}
