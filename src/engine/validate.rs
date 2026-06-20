use crate::{
    Alphabet, DecodeError, EncodeError, Engine, LineWrap, checked_encoded_len,
    checked_wrapped_encoded_len, decoded_len, encoded_len, validate_decode, validate_legacy_decode,
    validate_wrapped_decode, wrapped_encoded_len,
};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Returns the encoded length for this engine's padding policy.
    pub const fn encoded_len(&self, input_len: usize) -> Result<usize, EncodeError> {
        encoded_len(input_len, PAD)
    }

    /// Returns the encoded length for this engine, or `None` on overflow.
    #[must_use]
    pub const fn checked_encoded_len(&self, input_len: usize) -> Option<usize> {
        checked_encoded_len(input_len, PAD)
    }

    /// Returns the encoded length after applying a line wrapping policy.
    ///
    /// The returned length includes inserted line endings but does not include
    /// a trailing line ending after the final encoded line.
    pub const fn wrapped_encoded_len(
        &self,
        input_len: usize,
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        wrapped_encoded_len(input_len, PAD, wrap)
    }

    /// Returns the encoded length after line wrapping, or `None` on overflow or
    /// invalid line wrapping.
    #[must_use]
    pub const fn checked_wrapped_encoded_len(
        &self,
        input_len: usize,
        wrap: LineWrap,
    ) -> Option<usize> {
        checked_wrapped_encoded_len(input_len, PAD, wrap)
    }

    /// Returns the exact decoded length implied by input length and padding.
    ///
    /// This validates padding placement and impossible lengths, but it does not
    /// validate alphabet membership or non-canonical trailing bits.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        decoded_len(input, PAD)
    }

    /// Returns the exact decoded length for the explicit legacy profile.
    ///
    /// The legacy profile ignores ASCII space, tab, carriage return, and line
    /// feed bytes before applying the same alphabet, padding, and canonical-bit
    /// checks as strict decoding.
    pub fn decoded_len_legacy(&self, input: &[u8]) -> Result<usize, DecodeError> {
        validate_legacy_decode::<A, PAD>(input)
    }

    /// Returns the exact decoded length for a line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    pub fn decoded_len_wrapped(&self, input: &[u8], wrap: LineWrap) -> Result<usize, DecodeError> {
        validate_wrapped_decode::<A, PAD>(input, wrap)
    }

    /// Validates strict Base64 input without writing decoded bytes.
    ///
    /// This applies the same alphabet, padding, and canonical-bit checks as
    /// [`Self::decode_slice`]. Use this method when malformed-input
    /// diagnostics matter; use [`Self::validate`] when a boolean is enough.
    /// This default validator is not constant-time; use
    /// [`crate::ct::CtEngine::validate_result`] through [`Self::ct_decoder`]
    /// for secret-bearing payloads where timing posture matters.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// STANDARD.validate_result(b"aGVsbG8=").unwrap();
    /// assert!(STANDARD.validate_result(b"aGVsbG8").is_err());
    /// ```
    pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        validate_decode::<A, PAD>(input).map(|_| ())
    }

    /// Returns whether `input` is valid strict Base64 for this engine.
    ///
    /// This is a convenience wrapper around [`Self::validate_result`] and is
    /// not constant-time. Use [`crate::ct::CtEngine::validate`] through
    /// [`Self::ct_decoder`] for secret-bearing payloads where timing posture
    /// matters.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::URL_SAFE_NO_PAD;
    ///
    /// assert!(URL_SAFE_NO_PAD.validate(b"-_8"));
    /// assert!(!URL_SAFE_NO_PAD.validate(b"+/8"));
    /// ```
    #[must_use]
    pub fn validate(&self, input: &[u8]) -> bool {
        self.validate_result(input).is_ok()
    }

    /// Validates input using the explicit legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored
    /// before applying the same alphabet, padding, and canonical-bit checks as
    /// strict decoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// STANDARD.validate_legacy_result(b" aG\r\nVsbG8= ").unwrap();
    /// assert!(STANDARD.validate_legacy_result(b" aG-=").is_err());
    /// ```
    pub fn validate_legacy_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        validate_legacy_decode::<A, PAD>(input).map(|_| ())
    }

    /// Returns whether `input` is valid for the explicit legacy whitespace
    /// profile.
    ///
    /// This is a convenience wrapper around [`Self::validate_legacy_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// assert!(STANDARD.validate_legacy(b" aG\r\nVsbG8= "));
    /// assert!(!STANDARD.validate_legacy(b"aG-V"));
    /// ```
    #[must_use]
    pub fn validate_legacy(&self, input: &[u8]) -> bool {
        self.validate_legacy_result(input).is_ok()
    }

    /// Validates input using a strict line-wrapped profile.
    ///
    /// This is stricter than [`Self::validate_legacy_result`]: it accepts only
    /// the configured line ending and enforces the configured line length for
    /// every non-final line.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// STANDARD.validate_wrapped_result(b"aGVs\nbG8=", wrap).unwrap();
    /// assert!(STANDARD.validate_wrapped_result(b"aG\nVsbG8=", wrap).is_err());
    /// ```
    pub fn validate_wrapped_result(&self, input: &[u8], wrap: LineWrap) -> Result<(), DecodeError> {
        validate_wrapped_decode::<A, PAD>(input, wrap).map(|_| ())
    }

    /// Returns whether `input` is valid for a strict line-wrapped profile.
    ///
    /// This is a convenience wrapper around [`Self::validate_wrapped_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// assert!(STANDARD.validate_wrapped(b"aGVs\nbG8=", wrap));
    /// assert!(!STANDARD.validate_wrapped(b"aG\nVsbG8=", wrap));
    /// ```
    #[must_use]
    pub fn validate_wrapped(&self, input: &[u8], wrap: LineWrap) -> bool {
        self.validate_wrapped_result(input, wrap).is_ok()
    }
}
