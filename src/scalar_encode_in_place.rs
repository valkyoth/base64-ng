//! Scalar in-place encoding helper.

use crate::{Alphabet, EncodeError, checked_encoded_len, encode_base64_value_runtime};

pub(crate) fn encode_in_place<A, const PAD: bool>(
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

    let mut read = input_len;
    let mut write = required;

    match input_len % 3 {
        0 => {}
        1 => {
            read -= 1;
            let b0 = buffer[read];
            if PAD {
                write -= 4;
                buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                buffer[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
                buffer[write + 2] = b'=';
                buffer[write + 3] = b'=';
            } else {
                write -= 2;
                buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                buffer[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
            }
        }
        2 => {
            read -= 2;
            let b0 = buffer[read];
            let b1 = buffer[read + 1];
            if PAD {
                write -= 4;
                buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                buffer[write + 1] =
                    encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                buffer[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
                buffer[write + 3] = b'=';
            } else {
                write -= 3;
                buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                buffer[write + 1] =
                    encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                buffer[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
            }
        }
        _ => unreachable!(),
    }

    while read > 0 {
        read -= 3;
        write -= 4;
        let b0 = buffer[read];
        let b1 = buffer[read + 1];
        let b2 = buffer[read + 2];

        buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
        buffer[write + 1] = encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
        buffer[write + 2] = encode_base64_value_runtime::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
        buffer[write + 3] = encode_base64_value_runtime::<A>(b2 & 0b0011_1111);
    }

    // The right-to-left loop consumes exactly three input bytes for every four
    // output bytes. If this invariant changes, returning a shifted slice would
    // silently corrupt the in-place output.
    assert_eq!(
        write, 0,
        "encode_in_place invariant violated: right-to-left loop did not complete"
    );
    Ok(required)
}
