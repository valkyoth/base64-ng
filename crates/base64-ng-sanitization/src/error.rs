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

/// Error returned by fail-closed locked-secret decode helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockedDecodeError<E> {
    /// Locked allocation, integrity validation, or Base64 decoding failed.
    Operation(E),
    /// The mapping was created, but one or more requested protection controls
    /// were not established.
    DegradedProtection,
}

impl<E: core::fmt::Display> core::fmt::Display for LockedDecodeError<E> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Operation(error) => error.fmt(formatter),
            Self::DegradedProtection => {
                formatter.write_str("locked secret protection report is degraded")
            }
        }
    }
}

#[cfg(feature = "std")]
impl<E> std::error::Error for LockedDecodeError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Operation(error) => Some(error),
            Self::DegradedProtection => None,
        }
    }
}
