use base64_ng::DecodeError;

/// Error returned by fixed-size sanitization decode helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SanitizationDecodeError {
    /// The Base64 decoder rejected the input.
    Decode(DecodeError),
    /// Encoded input exceeds the public limit for the fixed-size destination.
    EncodedInputLimit {
        /// Maximum accepted encoded bytes.
        maximum: usize,
        /// Encoded bytes supplied by the caller.
        actual: usize,
    },
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
            Self::EncodedInputLimit { maximum, actual } => write!(
                formatter,
                "encoded secret exceeds limit: maximum {maximum} bytes, received {actual}"
            ),
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

/// Error returned by bounded heap-backed secret decode helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SecretVecDecodeError {
    /// The Base64 decoder rejected the input or staging capacity.
    Decode(DecodeError),
    /// Encoded input exceeds the public limit derived from output capacity.
    EncodedInputLimit {
        /// Maximum accepted encoded bytes.
        maximum: usize,
        /// Encoded bytes supplied by the caller.
        actual: usize,
    },
    /// Decoded output exceeds the caller-selected public capacity limit.
    CapacityLimit {
        /// Maximum accepted decoded bytes.
        maximum: usize,
        /// Decoded bytes required by the input.
        actual: usize,
    },
    /// The complete bounded output allocation could not be reserved.
    AllocationFailed {
        /// Number of decoded bytes requested.
        requested: usize,
    },
}

impl core::fmt::Display for SecretVecDecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(formatter),
            Self::EncodedInputLimit { maximum, actual } => write!(
                formatter,
                "encoded secret exceeds limit: maximum {maximum} bytes, received {actual}"
            ),
            Self::CapacityLimit { maximum, actual } => write!(
                formatter,
                "decoded secret exceeds limit: maximum {maximum} bytes, requires {actual}"
            ),
            Self::AllocationFailed { requested } => {
                write!(
                    formatter,
                    "failed to reserve {requested} decoded secret bytes"
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SecretVecDecodeError {}

impl From<DecodeError> for SecretVecDecodeError {
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
