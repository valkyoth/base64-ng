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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub(crate) fn with_index_offset(self, offset: usize) -> Self {
        match self {
            Self::InvalidByte { index, byte } => Self::InvalidByte {
                index: index + offset,
                byte,
            },
            Self::InvalidPadding { index } => Self::InvalidPadding {
                index: index + offset,
            },
            Self::InvalidLineWrap { index } => Self::InvalidLineWrap {
                index: index + offset,
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
