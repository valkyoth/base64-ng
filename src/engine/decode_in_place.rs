use crate::{
    Alphabet, DecodeError, Engine, LineWrap, compact_wrapped_input, decode_backend,
    is_legacy_whitespace, validate_decode, validate_legacy_decode, validate_wrapped_decode,
    wipe_bytes, wipe_tail,
};

const IN_PLACE_DECODE_INPUT_CHUNK: usize = 1024;

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
    /// may contain the whitespace-stripped encoded form of the input. This is
    /// still encoded material, not decoded plaintext, but it remains a modified
    /// representation of the original payload. On success, bytes after the
    /// returned decoded prefix may retain the compacted encoded representation.
    /// Use
    /// [`Self::decode_in_place_wrapped_clear_tail`] when the buffer may be
    /// reused or freed without a caller-managed wipe; treat that clear-tail
    /// variant as the default for secret-bearing wrapped payloads. If the
    /// original encoded input must be preserved for audit logging or retry,
    /// copy it before calling any in-place decode method or use a slice-output
    /// decode API instead.
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
    /// This default strict decoder prioritizes validation, exact error
    /// reporting, and ordinary throughput. It may branch or return early based
    /// on malformed input and reports exact failure positions and invalid byte
    /// values through [`DecodeError`]. For admitted Standard and URL-safe
    /// runtime profiles, successful decode may use stack staging before the
    /// strict decode backend writes behind the unread input cursor. Do not use
    /// this method for token comparison, key-material decoding, or
    /// secret-bearing validation where malformed-input timing matters. Do not
    /// log strict decode errors verbatim for secret-bearing input; log
    /// [`DecodeError::kind`] instead.
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
        let mut scratch = [0u8; IN_PLACE_DECODE_INPUT_CHUNK];
        let mut read = 0;
        let mut write = 0;

        while read < input_len {
            let chunk_len = in_place_decode_chunk_len(input_len - read);
            scratch[..chunk_len].copy_from_slice(&buffer[read..read + chunk_len]);
            let available = buffer.len();
            let Some(output_tail) = buffer.get_mut(write..) else {
                wipe_bytes(&mut scratch[..chunk_len]);
                return Err(DecodeError::OutputTooSmall {
                    required: write,
                    available,
                });
            };

            let written =
                match decode_backend::decode_slice::<A, PAD>(&scratch[..chunk_len], output_tail) {
                    Ok(written) => written,
                    Err(err) => {
                        wipe_bytes(&mut scratch[..chunk_len]);
                        return Err(err.with_index_offset(read));
                    }
                };
            wipe_bytes(&mut scratch[..chunk_len]);

            read += chunk_len;
            write += written;
            if written < decoded_chunk_max(chunk_len) {
                break;
            }
        }

        Ok(write)
    }
}

const fn in_place_decode_chunk_len(remaining: usize) -> usize {
    if remaining <= IN_PLACE_DECODE_INPUT_CHUNK {
        remaining
    } else {
        IN_PLACE_DECODE_INPUT_CHUNK
    }
}

const fn decoded_chunk_max(chunk_len: usize) -> usize {
    (chunk_len / 4) * 3
}
