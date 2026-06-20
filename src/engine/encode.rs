#[cfg(feature = "alloc")]
use crate::SecretBuffer;
use crate::{
    Alphabet, EncodeError, EncodedBuffer, Engine, LineWrap, checked_encoded_len,
    encode_base64_value, scalar, wipe_bytes, wipe_tail, write_wrapped_byte, write_wrapped_bytes,
};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Encodes a fixed-size input into a fixed-size output array in const contexts.
    ///
    /// Stable Rust does not yet allow this API to return an array whose length
    /// is computed from `INPUT_LEN` directly. Instead, the caller supplies the
    /// output length through the destination type and this function panics
    /// during const evaluation if the length is wrong.
    ///
    /// # Panics
    ///
    /// Panics if `OUTPUT_LEN` is not exactly the encoded length for `INPUT_LEN`
    /// and this engine's padding policy, or if that length overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// const HELLO: [u8; 8] = STANDARD.encode_array(b"hello");
    /// const URL_SAFE: [u8; 3] = URL_SAFE_NO_PAD.encode_array(b"\xfb\xff");
    ///
    /// assert_eq!(&HELLO, b"aGVsbG8=");
    /// assert_eq!(&URL_SAFE, b"-_8");
    /// ```
    ///
    /// Incorrect output lengths fail during const evaluation:
    ///
    /// ```compile_fail
    /// use base64_ng::STANDARD;
    ///
    /// const TOO_SHORT: [u8; 7] = STANDARD.encode_array(b"hello");
    /// ```
    #[must_use]
    pub const fn encode_array<const INPUT_LEN: usize, const OUTPUT_LEN: usize>(
        &self,
        input: &[u8; INPUT_LEN],
    ) -> [u8; OUTPUT_LEN] {
        let Some(required) = checked_encoded_len(INPUT_LEN, PAD) else {
            panic!("encoded base64 length overflows usize");
        };
        assert!(
            required == OUTPUT_LEN,
            "base64 output array has incorrect length"
        );

        let mut output = [0u8; OUTPUT_LEN];
        let mut read = 0;
        let mut write = 0;
        while INPUT_LEN - read >= 3 {
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

        match INPUT_LEN - read {
            0 => {}
            1 => {
                let b0 = input[read];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>((b0 & 0b0000_0011) << 4);
                write += 2;
                if PAD {
                    output[write] = b'=';
                    output[write + 1] = b'=';
                }
            }
            2 => {
                let b0 = input[read];
                let b1 = input[read + 1];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                output[write + 2] = encode_base64_value::<A>((b1 & 0b0000_1111) << 2);
                if PAD {
                    output[write + 3] = b'=';
                }
            }
            _ => unreachable!(),
        }

        output
    }

    /// Encodes `input` into `output`, returning the number of bytes written.
    pub fn encode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        scalar::encode_slice::<A, PAD>(input, output)
    }

    /// Encodes `input` into `output` with line wrapping.
    ///
    /// The wrapping policy inserts line endings between encoded lines and does
    /// not append a trailing line ending after the final line.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// let mut output = [0u8; 9];
    /// let written = STANDARD
    ///     .encode_slice_wrapped(b"hello", &mut output, wrap)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVs\nbG8=");
    /// ```
    pub fn encode_slice_wrapped(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        let required = self.wrapped_encoded_len(input.len(), wrap)?;
        if output.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }

        let encoded_len =
            checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        if encoded_len == 0 {
            return Ok(0);
        }

        // If the temporary in-buffer layout size overflows, fall back to the
        // fixed scratch buffer path rather than relying on saturated arithmetic.
        let combined_required = match required.checked_add(encoded_len) {
            Some(len) => len,
            None => usize::MAX,
        };
        if output.len() < combined_required {
            let mut scratch = [0u8; 1024];
            let mut input_offset = 0;
            let mut output_offset = 0;
            let mut column = 0;

            while input_offset < input.len() {
                let remaining = input.len() - input_offset;
                let mut take = remaining.min(768);
                if remaining > take {
                    take -= take % 3;
                }
                if take == 0 {
                    take = remaining;
                }

                let encoded = match self
                    .encode_slice(&input[input_offset..input_offset + take], &mut scratch)
                {
                    Ok(encoded) => encoded,
                    Err(err) => {
                        wipe_bytes(&mut scratch);
                        return Err(err);
                    }
                };
                if let Err(err) = write_wrapped_bytes(
                    &scratch[..encoded],
                    output,
                    &mut output_offset,
                    &mut column,
                    wrap,
                ) {
                    wipe_bytes(&mut scratch);
                    return Err(err);
                }
                wipe_bytes(&mut scratch[..encoded]);
                input_offset += take;
            }

            Ok(output_offset)
        } else {
            let encoded =
                self.encode_slice(input, &mut output[required..required + encoded_len])?;
            let mut output_offset = 0;
            let mut column = 0;
            let mut read = required;
            while read < required + encoded {
                let byte = output[read];
                write_wrapped_byte(byte, output, &mut output_offset, &mut column, wrap)?;
                read += 1;
            }
            wipe_bytes(&mut output[required..required + encoded]);
            Ok(output_offset)
        }
    }

    /// Encodes `input` with line wrapping and clears all bytes after the
    /// encoded prefix.
    ///
    /// If encoding fails, the entire output buffer is cleared before the error
    /// is returned.
    pub fn encode_slice_wrapped_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        let written = match self.encode_slice_wrapped(input, output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Encodes `input` with line wrapping into a stack-backed buffer.
    ///
    /// This is useful for MIME/PEM-style protocols where heap allocation is
    /// unnecessary. If encoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn encode_wrapped_buffer<const CAP: usize>(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<EncodedBuffer<CAP>, EncodeError> {
        let mut output = EncodedBuffer::new();
        let written =
            match self.encode_slice_wrapped_clear_tail(input, output.as_mut_capacity(), wrap) {
                Ok(written) => written,
                Err(err) => {
                    output.clear();
                    return Err(err);
                }
            };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Encodes `input` with line wrapping into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use encode_wrapped_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn encode_wrapped_vec(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = self.wrapped_encoded_len(input.len(), wrap)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice_wrapped(input, &mut output, wrap)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes `input` with line wrapping into a newly allocated UTF-8 string.
    #[cfg(feature = "alloc")]
    pub fn encode_wrapped_string(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::string::String, EncodeError> {
        let output = self.encode_wrapped_vec(input, wrap)?;
        match alloc::string::String::from_utf8(output) {
            Ok(output) => Ok(output),
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Encodes `input` with line wrapping into a redacted owned secret buffer.
    ///
    /// This is useful when the wrapped encoded representation itself is
    /// sensitive and should not be accidentally logged through formatting.
    #[cfg(feature = "alloc")]
    pub fn encode_wrapped_secret(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<SecretBuffer, EncodeError> {
        self.encode_wrapped_vec(input, wrap)
            .map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into `output` and clears all bytes after the encoded
    /// prefix.
    ///
    /// If encoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 12];
    /// let written = STANDARD
    ///     .encode_slice_clear_tail(b"hello", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVsbG8=");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn encode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        let written = match self.encode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Encodes `input` into a stack-backed buffer.
    ///
    /// This helper is useful for short values where callers want the
    /// convenience of an owned result without enabling `alloc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let encoded = STANDARD.encode_buffer::<8>(b"hello").unwrap();
    ///
    /// assert_eq!(encoded.as_str(), "aGVsbG8=");
    /// ```
    pub fn encode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<EncodedBuffer<CAP>, EncodeError> {
        let mut output = EncodedBuffer::new();
        let written = match self.encode_slice_clear_tail(input, output.as_mut_capacity()) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Encodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use encode_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn encode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice(input, &mut output)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes `input` into a redacted owned secret buffer.
    ///
    /// This is useful when the encoded representation itself is sensitive and
    /// should not be accidentally logged through formatting.
    #[cfg(feature = "alloc")]
    pub fn encode_secret(&self, input: &[u8]) -> Result<SecretBuffer, EncodeError> {
        self.encode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into a newly allocated UTF-8 string.
    ///
    /// Base64 output is ASCII by construction. This helper is available with
    /// the `alloc` feature and has the same encoding semantics as
    /// [`Self::encode_slice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// assert_eq!(STANDARD.encode_string(b"hello").unwrap(), "aGVsbG8=");
    /// assert_eq!(URL_SAFE_NO_PAD.encode_string(b"\xfb\xff").unwrap(), "-_8");
    /// ```
    #[cfg(feature = "alloc")]
    pub fn encode_string(&self, input: &[u8]) -> Result<alloc::string::String, EncodeError> {
        let output = self.encode_vec(input)?;
        match alloc::string::String::from_utf8(output) {
            Ok(output) => Ok(output),
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }
}
