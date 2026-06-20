//! Error types for encoding and decoding operations.

/// Encoding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The encoded output length would overflow `usize`.
    LengthOverflow,
    /// The requested line wrapping policy is invalid.
    InvalidLineWrap {
        /// Requested line length.
        line_len: usize,
    },
    /// The caller-provided input length exceeds the provided buffer.
    InputTooLarge {
        /// Requested input bytes.
        input_len: usize,
        /// Available buffer bytes.
        buffer_len: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LengthOverflow => f.write_str("base64 output length overflows usize"),
            Self::InvalidLineWrap { line_len } => {
                write!(f, "base64 line wrap length {line_len} is invalid")
            }
            Self::InputTooLarge {
                input_len,
                buffer_len,
            } => write!(
                f,
                "base64 input length {input_len} exceeds buffer length {buffer_len}"
            ),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 output buffer too small: required {required}, available {available}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

/// Decoding error.
///
/// # Security
///
/// Strict decoding errors are diagnostic values. Some variants carry
/// input-derived bytes or exact input indexes, and [`core::fmt::Display`]
/// intentionally prints those diagnostics for developer-facing debugging. Do
/// not log or return full [`DecodeError`] values for secret-bearing input; log
/// [`Self::kind`] instead.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum DecodeError {
    /// The encoded input is malformed, but the decoder intentionally does not
    /// disclose a more specific error class.
    InvalidInput,
    /// The encoded input length is impossible for the selected padding policy.
    InvalidLength,
    /// A byte is not valid for the selected alphabet.
    InvalidByte {
        /// Byte index in the input.
        index: usize,
        /// Invalid byte value.
        byte: u8,
    },
    /// Padding is missing, misplaced, or non-canonical.
    InvalidPadding {
        /// Byte index where padding became invalid.
        index: usize,
    },
    /// Line wrapping is missing, misplaced, or uses the wrong line ending.
    InvalidLineWrap {
        /// Byte index where line wrapping became invalid.
        index: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
    /// The caller-provided constant-time staging buffer is too small.
    StagingTooSmall {
        /// Required staging bytes.
        required: usize,
        /// Available staging bytes.
        available: usize,
    },
}

impl core::fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DecodeError")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

/// Redacted decoding error class.
///
/// This type intentionally omits input-derived bytes and indexes so callers can
/// log error classes without logging secret-adjacent input content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DecodeErrorKind {
    /// The encoded input is malformed, but the decoder intentionally does not
    /// disclose a more specific error class.
    InvalidInput,
    /// The encoded input length is impossible for the selected padding policy.
    InvalidLength,
    /// A byte is not valid for the selected alphabet.
    InvalidByte,
    /// Padding is missing, misplaced, or non-canonical.
    InvalidPadding,
    /// Line wrapping is missing, misplaced, or uses the wrong line ending.
    InvalidLineWrap,
    /// The output buffer is too small.
    OutputTooSmall,
    /// The caller-provided constant-time staging buffer is too small.
    StagingTooSmall,
}

impl DecodeErrorKind {
    /// Returns the stable lowercase identifier for this error class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid-input",
            Self::InvalidLength => "invalid-length",
            Self::InvalidByte => "invalid-byte",
            Self::InvalidPadding => "invalid-padding",
            Self::InvalidLineWrap => "invalid-line-wrap",
            Self::OutputTooSmall => "output-too-small",
            Self::StagingTooSmall => "staging-too-small",
        }
    }
}

impl core::fmt::Display for DecodeErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("malformed base64 input"),
            Self::InvalidLength => f.write_str("invalid base64 input length"),
            Self::InvalidByte { index, byte } => {
                write!(f, "invalid base64 byte 0x{byte:02x} at index {index}")
            }
            Self::InvalidPadding { index } => write!(f, "invalid base64 padding at index {index}"),
            Self::InvalidLineWrap { index } => {
                write!(f, "invalid base64 line wrapping at index {index}")
            }
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 decode output buffer too small: required {required}, available {available}"
            ),
            Self::StagingTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 decode staging buffer too small: required {required}, available {available}"
            ),
        }
    }
}

impl DecodeError {
    /// Returns a redacted error class without input-derived bytes or indexes.
    ///
    /// Strict decoders keep exact diagnostics in [`DecodeError`] and
    /// [`core::fmt::Display`] for developer debugging. When input may contain
    /// secrets or secret-adjacent material, log this kind instead of logging
    /// the full error value.
    #[must_use]
    pub const fn kind(self) -> DecodeErrorKind {
        match self {
            Self::InvalidInput => DecodeErrorKind::InvalidInput,
            Self::InvalidLength => DecodeErrorKind::InvalidLength,
            Self::InvalidByte { .. } => DecodeErrorKind::InvalidByte,
            Self::InvalidPadding { .. } => DecodeErrorKind::InvalidPadding,
            Self::InvalidLineWrap { .. } => DecodeErrorKind::InvalidLineWrap,
            Self::OutputTooSmall { .. } => DecodeErrorKind::OutputTooSmall,
            Self::StagingTooSmall { .. } => DecodeErrorKind::StagingTooSmall,
        }
    }

    pub(crate) fn with_index_offset(self, offset: usize) -> Self {
        match self {
            Self::InvalidByte { index, byte } => Self::InvalidByte {
                index: index.saturating_add(offset),
                byte,
            },
            Self::InvalidPadding { index } => Self::InvalidPadding {
                index: index.saturating_add(offset),
            },
            Self::InvalidLineWrap { index } => Self::InvalidLineWrap {
                index: index.saturating_add(offset),
            },
            Self::InvalidInput
            | Self::InvalidLength
            | Self::OutputTooSmall { .. }
            | Self::StagingTooSmall { .. } => self,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::DecodeError;

    #[test]
    fn index_offsets_saturate_on_overflow() {
        assert_eq!(
            DecodeError::InvalidByte {
                index: 7,
                byte: b'$'
            }
            .with_index_offset(usize::MAX),
            DecodeError::InvalidByte {
                index: usize::MAX,
                byte: b'$'
            }
        );
        assert_eq!(
            DecodeError::InvalidPadding { index: 7 }.with_index_offset(usize::MAX),
            DecodeError::InvalidPadding { index: usize::MAX }
        );
        assert_eq!(
            DecodeError::InvalidLineWrap { index: 7 }.with_index_offset(usize::MAX),
            DecodeError::InvalidLineWrap { index: usize::MAX }
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn debug_redacts_input_derived_details() {
        let error = DecodeError::InvalidByte {
            index: 42,
            byte: b'$',
        };
        let rendered = alloc::format!("{error:?}");
        assert!(rendered.contains("InvalidByte"));
        assert!(!rendered.contains("42"));
        assert!(!rendered.contains("24"));
    }
}
