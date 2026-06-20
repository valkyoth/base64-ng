use crate::{
    Alphabet, EncodeError, Engine, checked_encoded_len, encode_base64_value_runtime, wipe_bytes,
    wipe_tail,
};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Encodes the first `input_len` bytes of `buffer` in place.
    ///
    /// The buffer must have enough spare capacity for the encoded output. The
    /// implementation writes from right to left, so unread input bytes are not
    /// overwritten before they are encoded.
    ///
    /// # Panics
    ///
    /// Panics only if an internal right-to-left encode invariant is violated.
    /// This indicates a bug in `base64-ng`; valid or malformed caller input is
    /// reported through [`EncodeError`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0u8; 8];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
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
            buffer[write + 1] =
                encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            buffer[write + 2] =
                encode_base64_value_runtime::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            buffer[write + 3] = encode_base64_value_runtime::<A>(b2 & 0b0011_1111);
        }

        // The right-to-left loop consumes exactly three input bytes for every
        // four output bytes. If this invariant changes, returning a shifted
        // slice would silently corrupt the in-place output.
        assert_eq!(
            write, 0,
            "encode_in_place invariant violated: right-to-left loop did not complete"
        );
        Ok(&mut buffer[..required])
    }

    /// Encodes the first `input_len` bytes of `buffer` in place and clears all
    /// bytes after the encoded prefix.
    ///
    /// If encoding fails because `input_len` is too large, the output buffer is
    /// too small, or the encoded length overflows `usize`, the entire buffer is
    /// cleared before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0xff; 12];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place_clear_tail(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        let len = match self.encode_in_place(buffer, input_len) {
            Ok(encoded) => encoded.len(),
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }
}
