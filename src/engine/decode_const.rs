use crate::{Alphabet, DecodeError, Engine};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Decodes a fixed-size Base64 input into a fixed-size output array in
    /// const contexts.
    ///
    /// The returned tuple contains the output array and the number of decoded
    /// bytes written into that array. Bytes after the decoded prefix are zero.
    ///
    /// Unlike [`Engine::encode_array`](crate::Engine::encode_array), this
    /// function does not use panics for sizing mistakes. If `OUTPUT_CAP` is too
    /// small or the input is malformed, it returns [`DecodeError`]. This keeps
    /// the same function suitable for compile-time constants and runtime calls.
    ///
    /// # Security
    ///
    /// This is the normal strict decoder in const form. It is not a
    /// constant-time-oriented secret decoder, and strict errors may reveal
    /// input-derived indexes and bytes. Use [`crate::ct`] for sensitive decode
    /// timing posture.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// const DECODED: ([u8; 5], usize) = match STANDARD.decode_array(b"aGVsbG8=") {
    ///     Ok(decoded) => decoded,
    ///     Err(_) => panic!("static base64 literal should decode"),
    /// };
    ///
    /// assert_eq!(&DECODED.0[..DECODED.1], b"hello");
    /// ```
    pub const fn decode_array<const INPUT_LEN: usize, const OUTPUT_CAP: usize>(
        &self,
        input: &[u8; INPUT_LEN],
    ) -> Result<([u8; OUTPUT_CAP], usize), DecodeError> {
        let required = match const_decoded_len::<PAD>(input) {
            Ok(required) => required,
            Err(error) => return Err(error),
        };
        if OUTPUT_CAP < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: OUTPUT_CAP,
            });
        }

        let mut output = [0u8; OUTPUT_CAP];
        let written = if PAD {
            match const_decode_padded::<A, INPUT_LEN, OUTPUT_CAP>(input, &mut output) {
                Ok(written) => written,
                Err(error) => return Err(error),
            }
        } else {
            match const_decode_unpadded::<A, INPUT_LEN, OUTPUT_CAP>(input, &mut output) {
                Ok(written) => written,
                Err(error) => return Err(error),
            }
        };

        Ok((output, written))
    }
}

const fn const_decoded_len<const PAD: bool>(input: &[u8]) -> Result<usize, DecodeError> {
    if PAD {
        const_decoded_len_padded(input)
    } else {
        const_decoded_len_unpadded(input)
    }
}

const fn const_decoded_len_padded(input: &[u8]) -> Result<usize, DecodeError> {
    let len = input.len();
    if len == 0 {
        return Ok(0);
    }
    if len & 3 != 0 {
        return Err(DecodeError::InvalidLength);
    }

    let mut padding = 0;
    if input[len - 1] == b'=' {
        padding += 1;
    }
    if input[len - 2] == b'=' {
        padding += 1;
    }

    let first_pad = len - padding;
    let mut index = 0;
    while index < first_pad {
        if input[index] == b'=' {
            return Err(DecodeError::InvalidPadding { index });
        }
        index += 1;
    }

    Ok(len / 4 * 3 - padding)
}

const fn const_decoded_len_unpadded(input: &[u8]) -> Result<usize, DecodeError> {
    let len = input.len();
    let remainder = len & 3;
    if remainder == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let mut index = 0;
    while index < len {
        if input[index] == b'=' {
            return Err(DecodeError::InvalidPadding { index });
        }
        index += 1;
    }

    Ok(len / 4 * 3
        + if remainder == 2 {
            1
        } else if remainder == 3 {
            2
        } else {
            0
        })
}

const fn const_decode_padded<A: Alphabet, const INPUT_LEN: usize, const OUTPUT_CAP: usize>(
    input: &[u8; INPUT_LEN],
    output: &mut [u8; OUTPUT_CAP],
) -> Result<usize, DecodeError> {
    let mut read = 0;
    let mut write = 0;

    while read < INPUT_LEN {
        let written = match const_decode_quantum::<A, true, OUTPUT_CAP>(
            input[read],
            input[read + 1],
            input[read + 2],
            input[read + 3],
            read,
            output,
            write,
        ) {
            Ok(written) => written,
            Err(error) => return Err(error),
        };
        read += 4;
        write += written;
        if written < 3 && read != INPUT_LEN {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }

    Ok(write)
}

const fn const_decode_unpadded<A: Alphabet, const INPUT_LEN: usize, const OUTPUT_CAP: usize>(
    input: &[u8; INPUT_LEN],
    output: &mut [u8; OUTPUT_CAP],
) -> Result<usize, DecodeError> {
    let mut read = 0;
    let mut write = 0;

    while read + 4 <= INPUT_LEN {
        let written = match const_decode_quantum::<A, false, OUTPUT_CAP>(
            input[read],
            input[read + 1],
            input[read + 2],
            input[read + 3],
            read,
            output,
            write,
        ) {
            Ok(written) => written,
            Err(error) => return Err(error),
        };
        read += 4;
        write += written;
    }

    match INPUT_LEN - read {
        0 => Ok(write),
        2 => {
            let v0 = match const_decode_byte::<A>(input[read], read) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            let v1 = match const_decode_byte::<A>(input[read + 1], read + 1) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: read + 1 });
            }
            if let Err(error) = const_ensure_output::<OUTPUT_CAP>(write, 1) {
                return Err(error);
            }
            output[write] = (v0 << 2) | (v1 >> 4);
            Ok(write + 1)
        }
        3 => {
            let v0 = match const_decode_byte::<A>(input[read], read) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            let v1 = match const_decode_byte::<A>(input[read + 1], read + 1) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            let v2 = match const_decode_byte::<A>(input[read + 2], read + 2) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: read + 2 });
            }
            if let Err(error) = const_ensure_output::<OUTPUT_CAP>(write, 2) {
                return Err(error);
            }
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            Ok(write + 2)
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

const fn const_decode_quantum<A: Alphabet, const PAD: bool, const OUTPUT_CAP: usize>(
    b0: u8,
    b1: u8,
    b2: u8,
    b3: u8,
    input_offset: usize,
    output: &mut [u8; OUTPUT_CAP],
    write: usize,
) -> Result<usize, DecodeError> {
    let v0 = match const_decode_byte::<A>(b0, input_offset) {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    let v1 = match const_decode_byte::<A>(b1, input_offset + 1) {
        Ok(value) => value,
        Err(error) => return Err(error),
    };

    match (b2, b3) {
        (b'=', b'=') if PAD => {
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding {
                    index: input_offset + 1,
                });
            }
            if let Err(error) = const_ensure_output::<OUTPUT_CAP>(write, 1) {
                return Err(error);
            }
            output[write] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding {
            index: input_offset + 2,
        }),
        (_, b'=') if PAD => {
            let v2 = match const_decode_byte::<A>(b2, input_offset + 2) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding {
                    index: input_offset + 2,
                });
            }
            if let Err(error) = const_ensure_output::<OUTPUT_CAP>(write, 2) {
                return Err(error);
            }
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        (b'=', _) => Err(DecodeError::InvalidPadding {
            index: input_offset + 2,
        }),
        (_, b'=') => Err(DecodeError::InvalidPadding {
            index: input_offset + 3,
        }),
        _ => {
            let v2 = match const_decode_byte::<A>(b2, input_offset + 2) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            let v3 = match const_decode_byte::<A>(b3, input_offset + 3) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            if let Err(error) = const_ensure_output::<OUTPUT_CAP>(write, 3) {
                return Err(error);
            }
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            output[write + 2] = (v2 << 6) | v3;
            Ok(3)
        }
    }
}

const fn const_ensure_output<const OUTPUT_CAP: usize>(
    write: usize,
    needed: usize,
) -> Result<(), DecodeError> {
    if write > OUTPUT_CAP || OUTPUT_CAP - write < needed {
        let required = if write > usize::MAX - needed {
            usize::MAX
        } else {
            write + needed
        };
        return Err(DecodeError::OutputTooSmall {
            required,
            available: OUTPUT_CAP,
        });
    }

    Ok(())
}

const fn const_decode_byte<A: Alphabet>(byte: u8, index: usize) -> Result<u8, DecodeError> {
    let mut candidate = 0u8;
    while candidate < 64 {
        if byte == A::ENCODE[candidate as usize] {
            return Ok(candidate);
        }
        candidate += 1;
    }

    Err(DecodeError::InvalidByte { index, byte })
}
