#[cfg(feature = "alloc")]
use crate::SecretBuffer;
#[cfg(feature = "alloc")]
use crate::validate_decode;
use crate::{
    Alphabet, DecodeError, DecodedBuffer, Engine, LineWrap, decode_backend, is_legacy_whitespace,
    validate_legacy_decode, validate_wrapped_decode, wipe_bytes, wipe_tail,
};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Decodes `input` into `output`, returning the number of bytes written.
    ///
    /// This is strict decoding. Whitespace, mixed alphabets, malformed padding,
    /// and trailing non-padding data are rejected.
    ///
    /// # Security
    ///
    /// This default scalar decoder prioritizes strict validation, exact error
    /// reporting, and ordinary throughput. It may branch or return early based
    /// on byte validity, malformed input, padding position, and output
    /// capacity. It also reports exact failure positions and invalid byte
    /// values through [`DecodeError`]. Do not use this method for token
    /// comparison, key-material decoding, or secret-bearing validation where
    /// malformed-input timing matters. Do not log strict decode errors
    /// verbatim for secret-bearing input; log [`DecodeError::kind`] instead.
    /// Use [`crate::ct`],
    /// [`crate::ct::STANDARD`], [`crate::ct::URL_SAFE_NO_PAD`], or
    /// [`Self::ct_decoder`] with `decode_slice_clear_tail` for
    /// constant-time-oriented secret decoding.
    #[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
    pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        decode_backend::decode_slice::<A, PAD>(input, output)
    }

    /// Decodes `input` into `output` and clears all bytes after the decoded
    /// prefix.
    ///
    /// If decoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_clear_tail(b"aGk=", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` into a stack-backed buffer.
    ///
    /// This helper is useful for short decoded values where callers want the
    /// convenience of an owned result without enabling `alloc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let decoded = STANDARD.decode_buffer::<5>(b"aGVsbG8=").unwrap();
    ///
    /// assert_eq!(decoded.as_bytes(), b"hello");
    /// ```
    pub fn decode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_clear_tail(input, output.as_mut_capacity()) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Decodes `input` using the explicit legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict.
    ///
    /// # Security
    ///
    /// This method uses the normal strict decode path after legacy whitespace
    /// handling. It may branch or return early based on malformed input and is
    /// not a constant-time token validator or key-material decoder. Use
    /// [`crate::ct`] for secret-bearing payloads.
    #[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
    pub fn decode_slice_legacy(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        Self::decode_legacy_via_strict_backend(input, output)
    }

    /// Decodes `input` using the explicit legacy whitespace profile and clears
    /// all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire output buffer is cleared
    /// before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_legacy_clear_tail(b" aG\r\nk= ", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_legacy_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice_legacy(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` into a stack-backed buffer using the explicit legacy
    /// whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict. If decoding fails, the
    /// internal backing array is cleared before the error is returned.
    pub fn decode_buffer_legacy<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_legacy_clear_tail(input, output.as_mut_capacity()) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Decodes `input` using a strict line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    ///
    /// # Security
    ///
    /// This method uses the normal strict decode path after line-profile
    /// validation. It may branch or return early based on malformed input and
    /// is not a constant-time token validator or key-material decoder. Use
    /// [`crate::ct`] for secret-bearing payloads.
    #[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
    pub fn decode_slice_wrapped(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, DecodeError> {
        let required = validate_wrapped_decode::<A, PAD>(input, wrap)?;
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        Self::decode_wrapped_via_strict_backend(input, output, wrap)
    }

    /// Decodes `input` using a strict line-wrapped profile and clears all bytes
    /// after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire output buffer is cleared
    /// before the error is returned.
    pub fn decode_slice_wrapped_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice_wrapped(input, output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` using a strict line-wrapped profile into a stack-backed
    /// buffer.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted. If decoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn decode_wrapped_buffer<const CAP: usize>(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written =
            match self.decode_slice_wrapped_clear_tail(input, output.as_mut_capacity(), wrap) {
                Ok(written) => written,
                Err(err) => {
                    output.clear();
                    return Err(err);
                }
            };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Decodes `input` into a newly allocated byte vector.
    ///
    /// This is strict decoding with the same semantics as [`Self::decode_slice`].
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a redacted owned secret buffer.
    ///
    /// On malformed input, the intermediate output buffer is cleared before the
    /// error is returned by [`Self::decode_vec`].
    #[cfg(feature = "alloc")]
    pub fn decode_secret(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Decodes `input` into a newly allocated byte vector using the explicit
    /// legacy whitespace profile.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_secret_legacy, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_vec_legacy(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice_legacy(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a redacted owned secret buffer using the explicit
    /// legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict.
    #[cfg(feature = "alloc")]
    pub fn decode_secret_legacy(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec_legacy(input).map(SecretBuffer::from_vec)
    }

    /// Decodes line-wrapped input into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_wrapped_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_wrapped_vec(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_wrapped_decode::<A, PAD>(input, wrap)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice_wrapped(input, &mut output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes line-wrapped input into a redacted owned secret buffer.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    #[cfg(feature = "alloc")]
    pub fn decode_wrapped_secret(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<SecretBuffer, DecodeError> {
        self.decode_wrapped_vec(input, wrap)
            .map(SecretBuffer::from_vec)
    }

    fn decode_wrapped_via_strict_backend(
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, DecodeError> {
        let line_ending = wrap.line_ending.as_bytes();
        let mut scratch = [0u8; 1024];
        let mut scratch_indexes = [0usize; 1024];
        let mut scratch_len = 0;
        let mut read = 0;
        let mut write = 0;

        while read < input.len() {
            let line_end = read
                .checked_add(line_ending.len())
                .filter(|end| *end <= input.len());
            if line_end.and_then(|end| input.get(read..end)) == Some(line_ending) {
                read += line_ending.len();
                continue;
            }

            scratch[scratch_len] = input[read];
            scratch_indexes[scratch_len] = read;
            scratch_len += 1;
            read += 1;

            if scratch_len == scratch.len() {
                Self::decode_strict_scratch_chunk(
                    &mut scratch,
                    &scratch_indexes,
                    scratch_len,
                    output,
                    &mut write,
                )?;
                scratch_len = 0;
            }
        }

        if scratch_len == 0 {
            return Ok(write);
        }

        Self::decode_strict_scratch_chunk(
            &mut scratch,
            &scratch_indexes,
            scratch_len,
            output,
            &mut write,
        )?;
        Ok(write)
    }

    fn decode_legacy_via_strict_backend(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let mut scratch = [0u8; 1024];
        let mut scratch_indexes = [0usize; 1024];
        let mut scratch_len = 0;
        let mut write = 0;

        for (index, byte) in input.iter().enumerate() {
            if is_legacy_whitespace(*byte) {
                continue;
            }

            scratch[scratch_len] = *byte;
            scratch_indexes[scratch_len] = index;
            scratch_len += 1;

            if scratch_len == scratch.len() {
                Self::decode_strict_scratch_chunk(
                    &mut scratch,
                    &scratch_indexes,
                    scratch_len,
                    output,
                    &mut write,
                )?;
                scratch_len = 0;
            }
        }

        if scratch_len != 0 {
            Self::decode_strict_scratch_chunk(
                &mut scratch,
                &scratch_indexes,
                scratch_len,
                output,
                &mut write,
            )?;
        }

        Ok(write)
    }

    fn decode_strict_scratch_chunk(
        scratch: &mut [u8; 1024],
        scratch_indexes: &[usize; 1024],
        scratch_len: usize,
        output: &mut [u8],
        write: &mut usize,
    ) -> Result<(), DecodeError> {
        let available = output.len();
        let Some(output_tail) = output.get_mut(*write..) else {
            wipe_bytes(&mut scratch[..scratch_len]);
            return Err(DecodeError::OutputTooSmall {
                required: *write,
                available,
            });
        };
        let written =
            match decode_backend::decode_slice::<A, PAD>(&scratch[..scratch_len], output_tail) {
                Ok(written) => written,
                Err(err) => {
                    wipe_bytes(&mut scratch[..scratch_len]);
                    return Err(err.with_index_map(&scratch_indexes[..scratch_len]));
                }
            };
        wipe_bytes(&mut scratch[..scratch_len]);
        *write += written;
        Ok(())
    }
}
