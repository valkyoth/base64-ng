use base64_ng::DecodeError;

/// Error returned by fixed-size sanitization decode helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SanitizationDecodeError {
    /// The Base64 decoder rejected the input.
    Decode(DecodeError),
    /// The decoded byte length does not match the requested fixed-size secret.
    LengthMismatch {
        /// Expected decoded byte length.
        expected: usize,
        /// Actual decoded byte length.
        actual: usize,
    },
}

impl core::fmt::Display for SanitizationDecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(formatter),
            Self::LengthMismatch { expected, actual } => write!(
                formatter,
                "decoded Base64 length mismatch: expected {expected}, actual {actual}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SanitizationDecodeError {}

impl From<DecodeError> for SanitizationDecodeError {
    #[inline]
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}
