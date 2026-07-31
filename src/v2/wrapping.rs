//! Validated line-wrapping policy for the 2.0 codec core.

use core::num::NonZeroUsize;

/// Line ending inserted between encoded body lines.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum LineEnding {
    /// Line feed (`\n`).
    Lf,
    /// Carriage return followed by line feed (`\r\n`).
    CrLf,
}

impl LineEnding {
    /// Returns the exact line-ending bytes.
    pub(crate) const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Lf => b"\n",
            Self::CrLf => b"\r\n",
        }
    }

    /// Returns the line-ending width in bytes.
    pub(crate) const fn byte_len(self) -> usize {
        self.as_bytes().len()
    }
}

/// Failure constructing a line-wrapping policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineWrapError {
    /// A zero-width line would prevent encoder progress.
    ZeroWidth,
}

impl core::fmt::Display for LineWrapError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroWidth => formatter.write_str("base64 line width must be non-zero"),
        }
    }
}

/// Immutable, always-progressing Base64 body wrapping policy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct LineWrap {
    line_width: NonZeroUsize,
    line_ending: LineEnding,
}

impl LineWrap {
    /// MIME transfer-body wrapping: 76 columns with CRLF separators.
    ///
    /// This is a body-layout value, not a complete MIME parser or profile.
    pub(crate) const MIME_BODY_WRAP: Self = Self {
        line_width: NonZeroUsize::MIN.saturating_add(75),
        line_ending: LineEnding::CrLf,
    };

    /// PEM body wrapping: 64 columns with LF separators.
    ///
    /// This is a body-layout value, not a complete RFC 7468 parser.
    pub(crate) const PEM_BODY_LF_WRAP: Self = Self {
        line_width: NonZeroUsize::MIN.saturating_add(63),
        line_ending: LineEnding::Lf,
    };

    /// PEM body wrapping: 64 columns with CRLF separators.
    ///
    /// This is a body-layout value, not a complete RFC 7468 parser.
    pub(crate) const PEM_BODY_CRLF_WRAP: Self = Self {
        line_width: NonZeroUsize::MIN.saturating_add(63),
        line_ending: LineEnding::CrLf,
    };

    /// Constructs a validated wrapping policy.
    pub(crate) const fn try_new(
        line_width: usize,
        line_ending: LineEnding,
    ) -> Result<Self, LineWrapError> {
        match NonZeroUsize::new(line_width) {
            Some(line_width) => Ok(Self {
                line_width,
                line_ending,
            }),
            None => Err(LineWrapError::ZeroWidth),
        }
    }

    /// Returns the non-zero encoded body width.
    pub(crate) const fn line_width(self) -> NonZeroUsize {
        self.line_width
    }

    /// Returns the separator inserted between body lines.
    pub(crate) const fn line_ending(self) -> LineEnding {
        self.line_ending
    }

    /// Returns the exact wrapped size without a trailing line ending.
    pub(crate) const fn checked_output_len(self, payload_len: usize) -> Option<usize> {
        if payload_len == 0 {
            return Some(0);
        }

        let breaks = (payload_len - 1) / self.line_width.get();
        let Some(separator_bytes) = breaks.checked_mul(self.line_ending.byte_len()) else {
            return None;
        };
        payload_len.checked_add(separator_bytes)
    }

    /// Inserts line endings into an already encoded Base64 body.
    ///
    /// The destination is unchanged when it is too small or length arithmetic
    /// overflows. Successful output never has a trailing line ending.
    pub(crate) fn insert_into(self, payload: &[u8], output: &mut [u8]) -> Option<usize> {
        let required = self.checked_output_len(payload.len())?;
        if output.len() < required {
            return None;
        }

        let separator = self.line_ending.as_bytes();
        let width = self.line_width.get();
        let mut read = 0usize;
        let mut write = 0usize;
        let mut column = 0usize;
        while read < payload.len() {
            if column == width {
                let end = write.checked_add(separator.len())?;
                output[write..end].copy_from_slice(separator);
                write = end;
                column = 0;
            }
            output[write] = payload[read];
            read += 1;
            write += 1;
            column += 1;
        }
        Some(write)
    }

    /// Validates wrapped body layout and returns its unwrapped payload length.
    ///
    /// Interior lines must have exactly the configured width. A final line may
    /// be shorter, and one final line ending is accepted for compatibility
    /// with body formats that terminate their last line.
    pub(crate) fn payload_len(self, input: &[u8]) -> Option<usize> {
        let separator = self.line_ending.as_bytes();
        let width = self.line_width.get();
        let mut index = 0usize;
        let mut column = 0;
        let mut payload_len = 0;

        while index < input.len() {
            if starts_with(input, index, separator) {
                if column == 0 {
                    return None;
                }
                index = index.checked_add(separator.len())?;
                if index == input.len() {
                    return Some(payload_len);
                }
                if column != width {
                    return None;
                }
                column = 0;
                continue;
            }

            if matches!(input[index], b'\r' | b'\n') || column == width {
                return None;
            }
            index += 1;
            column += 1;
            payload_len += 1;
        }
        Some(payload_len)
    }

    /// Validates and copies a wrapped body without its line endings.
    ///
    /// The destination is unchanged when layout validation fails or the
    /// destination is too small.
    pub(crate) fn copy_payload_into(self, input: &[u8], output: &mut [u8]) -> Option<usize> {
        let payload_len = self.payload_len(input)?;
        if output.len() < payload_len {
            return None;
        }

        let separator = self.line_ending.as_bytes();
        let mut read = 0usize;
        let mut write = 0usize;
        while read < input.len() {
            if starts_with(input, read, separator) {
                read = read.checked_add(separator.len())?;
            } else {
                output[write] = input[read];
                read += 1;
                write += 1;
            }
        }
        Some(write)
    }
}

fn starts_with(input: &[u8], index: usize, needle: &[u8]) -> bool {
    let Some(end) = index.checked_add(needle.len()) else {
        return false;
    };
    end <= input.len() && &input[index..end] == needle
}
