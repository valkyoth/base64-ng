#[cfg(feature = "alloc")]
use crate::SecretBuffer;
#[cfg(feature = "alloc")]
use crate::validate_decode;
use crate::{
    Alphabet, DecodeError, DecodedBuffer, Engine, LineWrap, decode_legacy_to_slice,
    decode_wrapped_to_slice, scalar, validate_legacy_decode, validate_wrapped_decode, wipe_bytes,
    wipe_tail,
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
        scalar::decode_slice::<A, PAD>(input, output)
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
        decode_legacy_to_slice::<A, PAD>(input, output)
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
        decode_wrapped_to_slice::<A, PAD>(input, output, wrap)
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
}
