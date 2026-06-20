use crate::{
    Alphabet, DecodeError, Engine, LineWrap, compact_wrapped_input, decode_chunk,
    decode_tail_unpadded, is_legacy_whitespace, read_quad, validate_decode, validate_legacy_decode,
    validate_wrapped_decode, wipe_bytes, wipe_tail,
};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Decodes `buffer` in place using a strict line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    ///
    /// # Security
    ///
    /// This method compacts line endings in place before decoding. If
    /// validation or decoding fails, the buffer contents are unspecified and
    /// may contain a compacted encoded prefix. On success, bytes after the
    /// returned decoded prefix may retain the compacted encoded representation.
    /// Use
    /// [`Self::decode_in_place_wrapped_clear_tail`] when the buffer may be
    /// reused or freed without a caller-managed wipe; treat that clear-tail
    /// variant as the default for secret-bearing wrapped payloads.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let decoded = STANDARD
    ///     .decode_in_place_wrapped(&mut buffer, LineWrap::new(4, LineEnding::Lf))
    ///     .unwrap();
    ///
    /// assert_eq!(decoded, b"hello");
    /// ```
    pub fn decode_in_place_wrapped<'a>(
        &self,
        buffer: &'a mut [u8],
        wrap: LineWrap,
    ) -> Result<&'a mut [u8], DecodeError> {
        let _required = validate_wrapped_decode::<A, PAD>(buffer, wrap)?;
        let compacted = compact_wrapped_input(buffer, wrap)?;
        let len = Self::decode_slice_to_start(&mut buffer[..compacted])?;
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using a strict line-wrapped profile and clears
    /// all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let len = STANDARD
    ///     .decode_in_place_wrapped_clear_tail(&mut buffer, LineWrap::new(4, LineEnding::Lf))
    ///     .unwrap()
    ///     .len();
    ///
    /// assert_eq!(&buffer[..len], b"hello");
    /// assert!(buffer[len..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_in_place_wrapped_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
        wrap: LineWrap,
    ) -> Result<&'a mut [u8], DecodeError> {
        if let Err(err) = validate_wrapped_decode::<A, PAD>(buffer, wrap) {
            wipe_bytes(buffer);
            return Err(err);
        }

        let compacted = match compact_wrapped_input(buffer, wrap) {
            Ok(compacted) => compacted,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };

        let len = match Self::decode_slice_to_start(&mut buffer[..compacted]) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes the buffer in place and returns the decoded prefix.
    ///
    /// On success, bytes after the returned decoded prefix may retain encoded
    /// input bytes. Use [`Self::decode_in_place_clear_tail`] when the buffer
    /// may be reused or freed without a caller-managed wipe.
    ///
    /// # Security
    ///
    /// This default scalar decoder prioritizes strict validation, exact error
    /// reporting, and ordinary throughput. It may branch or return early based
    /// on malformed input and reports exact failure positions and invalid byte
    /// values through [`DecodeError`]. Do not use this method for token
    /// comparison, key-material decoding, or secret-bearing validation where
    /// malformed-input timing matters. Do not log strict decode errors
    /// verbatim for secret-bearing input; log [`DecodeError::kind`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD_NO_PAD;
    ///
    /// let mut buffer = *b"Zm9vYmFy";
    /// let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"foobar");
    /// ```
    pub fn decode_in_place<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], DecodeError> {
        let len = Self::decode_slice_to_start(buffer)?;
        Ok(&mut buffer[..len])
    }

    /// Decodes the buffer in place and clears all bytes after the decoded prefix.
    ///
    /// If decoding fails, the entire buffer is cleared before the error is
    /// returned. Use this variant when the encoded or partially decoded data is
    /// sensitive and the caller wants best-effort cleanup without adding a
    /// dependency.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = *b"aGk=";
    /// let decoded = STANDARD.decode_in_place_clear_tail(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"hi");
    /// ```
    pub fn decode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let len = match Self::decode_slice_to_start(buffer) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile.
    ///
    /// Ignored whitespace is compacted out before decoding. If validation
    /// fails, the buffer contents are unspecified. On success, bytes after the
    /// returned decoded prefix may retain the compacted encoded
    /// representation. Use [`Self::decode_in_place_legacy_clear_tail`] when the
    /// buffer may be reused or freed without a caller-managed wipe.
    pub fn decode_in_place_legacy<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let _required = validate_legacy_decode::<A, PAD>(buffer)?;
        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }
        let len = Self::decode_slice_to_start(&mut buffer[..write])?;
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile
    /// and clears all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    pub fn decode_in_place_legacy_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        if let Err(err) = validate_legacy_decode::<A, PAD>(buffer) {
            wipe_bytes(buffer);
            return Err(err);
        }

        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }

        let len = match Self::decode_slice_to_start(&mut buffer[..write]) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    fn decode_slice_to_start(buffer: &mut [u8]) -> Result<usize, DecodeError> {
        let _required = validate_decode::<A, PAD>(buffer)?;
        let input_len = buffer.len();
        let mut read = 0;
        let mut write = 0;
        while read + 4 <= input_len {
            let chunk = read_quad(buffer, read)?;
            let available = buffer.len();
            let output_tail = buffer.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
                required: write,
                available,
            })?;
            let written = decode_chunk::<A, PAD>(chunk, output_tail)
                .map_err(|err| err.with_index_offset(read))?;
            read += 4;
            write += written;
            if written < 3 {
                if read != input_len {
                    return Err(DecodeError::InvalidPadding { index: read - 4 });
                }
                return Ok(write);
            }
        }

        let rem = input_len - read;
        if rem == 0 {
            return Ok(write);
        }
        if PAD {
            return Err(DecodeError::InvalidLength);
        }
        let mut tail = [0u8; 3];
        tail[..rem].copy_from_slice(&buffer[read..input_len]);
        decode_tail_unpadded::<A>(&tail[..rem], &mut buffer[write..])
            .map_err(|err| err.with_index_offset(read))
            .map(|n| write + n)
    }
}
