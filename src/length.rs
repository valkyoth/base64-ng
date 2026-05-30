//! Length calculation and line wrapping policy helpers.

use crate::{DecodeError, EncodeError};

/// Line ending used by wrapped Base64 output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineEnding {
    /// Line feed (`\n`).
    Lf,
    /// Carriage return followed by line feed (`\r\n`).
    CrLf,
}

impl LineEnding {
    /// Returns a stable printable identifier for this line ending.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
        }
    }

    /// Returns the text representation of this line ending.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }

    /// Returns the byte representation of this line ending.
    #[must_use]
    pub const fn as_bytes(self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    /// Returns the byte length of this line ending.
    #[must_use]
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Lf => 1,
            Self::CrLf => 2,
        }
    }
}

impl core::fmt::Display for LineEnding {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Base64 line wrapping policy.
///
/// `line_len` is measured in encoded Base64 bytes, not source input bytes.
/// Encoders insert line endings between lines and do not append a trailing line
/// ending after the final line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineWrap {
    /// Maximum encoded bytes per line.
    pub line_len: usize,
    /// Line ending inserted between wrapped lines.
    pub line_ending: LineEnding,
}

impl LineWrap {
    /// MIME-style wrapping: 76 columns with CRLF endings.
    pub const MIME: Self = Self::new(76, LineEnding::CrLf);
    /// PEM-style wrapping: 64 columns with LF endings.
    pub const PEM: Self = Self::new(64, LineEnding::Lf);
    /// PEM-style wrapping: 64 columns with CRLF endings.
    pub const PEM_CRLF: Self = Self::new(64, LineEnding::CrLf);

    /// Creates a wrapping policy.
    ///
    /// This constructor is intended for fixed, trusted values such as
    /// compile-time MIME or PEM profile constants. Use [`Self::checked_new`]
    /// when the line length comes from configuration, network input, file
    /// metadata, or another untrusted runtime source.
    ///
    /// # Panics
    ///
    /// Panics when `line_len` is zero. Base64 wrapping requires a non-zero
    /// encoded line length; accepting zero would make progress impossible for
    /// wrapped encoders. This constructor is callable at runtime, so do not
    /// pass attacker-controlled or externally configured values here; use
    /// [`Self::checked_new`] for those cases.
    #[must_use]
    pub const fn new(line_len: usize, line_ending: LineEnding) -> Self {
        assert!(line_len != 0, "base64 line wrap length must be non-zero");
        Self {
            line_len,
            line_ending,
        }
    }

    /// Creates a wrapping policy, returning `None` when the line length is
    /// invalid.
    ///
    /// Base64 line-wrapping requires a non-zero encoded line length. This
    /// helper is useful when accepting a wrapping policy from configuration or
    /// another untrusted source.
    #[must_use]
    pub const fn checked_new(line_len: usize, line_ending: LineEnding) -> Option<Self> {
        if line_len == 0 {
            None
        } else {
            Some(Self::new(line_len, line_ending))
        }
    }

    /// Returns the maximum encoded bytes per line.
    #[must_use]
    pub const fn line_len(self) -> usize {
        self.line_len
    }

    /// Returns the line ending inserted between wrapped lines.
    #[must_use]
    pub const fn line_ending(self) -> LineEnding {
        self.line_ending
    }

    /// Returns whether this wrapping policy can be used by the encoder.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.line_len != 0
    }
}

impl core::fmt::Display for LineWrap {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}:{}", self.line_len, self.line_ending.name())
    }
}

/// Returns the encoded length for an input length and padding policy.
///
/// This function returns [`EncodeError::LengthOverflow`] instead of panicking.
/// Use [`checked_encoded_len`] when an `Option<usize>` is more convenient.
///
/// # Examples
///
/// ```
/// use base64_ng::encoded_len;
///
/// assert_eq!(encoded_len(5, true).unwrap(), 8);
/// assert_eq!(encoded_len(5, false).unwrap(), 7);
/// assert!(encoded_len(usize::MAX, true).is_err());
/// ```
pub const fn encoded_len(input_len: usize, padded: bool) -> Result<usize, EncodeError> {
    match checked_encoded_len(input_len, padded) {
        Some(len) => Ok(len),
        None => Err(EncodeError::LengthOverflow),
    }
}

/// Returns the encoded length after applying a line wrapping policy.
///
/// The returned length includes inserted line endings but does not include a
/// trailing line ending after the final encoded line.
///
/// # Examples
///
/// ```
/// use base64_ng::{LineEnding, LineWrap, wrapped_encoded_len};
///
/// let wrap = LineWrap::new(4, LineEnding::Lf);
/// assert_eq!(wrapped_encoded_len(5, true, wrap).unwrap(), 9);
/// ```
pub const fn wrapped_encoded_len(
    input_len: usize,
    padded: bool,
    wrap: LineWrap,
) -> Result<usize, EncodeError> {
    if wrap.line_len == 0 {
        return Err(EncodeError::InvalidLineWrap { line_len: 0 });
    }

    let Some(encoded) = checked_encoded_len(input_len, padded) else {
        return Err(EncodeError::LengthOverflow);
    };
    if encoded == 0 {
        return Ok(0);
    }

    let breaks = (encoded - 1) / wrap.line_len;
    let Some(line_ending_bytes) = breaks.checked_mul(wrap.line_ending.byte_len()) else {
        return Err(EncodeError::LengthOverflow);
    };
    match encoded.checked_add(line_ending_bytes) {
        Some(len) => Ok(len),
        None => Err(EncodeError::LengthOverflow),
    }
}

/// Returns the encoded length after line wrapping, or `None` on overflow or
/// invalid line wrapping.
///
/// The returned length includes inserted line endings but does not include a
/// trailing line ending after the final encoded line.
///
/// # Examples
///
/// ```
/// use base64_ng::{LineEnding, LineWrap, checked_wrapped_encoded_len};
///
/// let wrap = LineWrap::new(4, LineEnding::Lf);
/// assert_eq!(checked_wrapped_encoded_len(5, true, wrap), Some(9));
/// assert_eq!(LineWrap::checked_new(0, LineEnding::Lf), None);
/// ```
#[must_use]
pub const fn checked_wrapped_encoded_len(
    input_len: usize,
    padded: bool,
    wrap: LineWrap,
) -> Option<usize> {
    if wrap.line_len == 0 {
        return None;
    }

    let Some(encoded) = checked_encoded_len(input_len, padded) else {
        return None;
    };
    if encoded == 0 {
        return Some(0);
    }

    let breaks = (encoded - 1) / wrap.line_len;
    let Some(line_ending_bytes) = breaks.checked_mul(wrap.line_ending.byte_len()) else {
        return None;
    };
    encoded.checked_add(line_ending_bytes)
}

/// Returns the encoded length, or `None` if it would overflow `usize`.
///
/// # Examples
///
/// ```
/// use base64_ng::checked_encoded_len;
///
/// assert_eq!(checked_encoded_len(5, true), Some(8));
/// assert_eq!(checked_encoded_len(usize::MAX, true), None);
/// ```
#[must_use]
pub const fn checked_encoded_len(input_len: usize, padded: bool) -> Option<usize> {
    let groups = input_len / 3;
    if groups > usize::MAX / 4 {
        return None;
    }
    let full = groups * 4;
    let rem = input_len % 3;
    if rem == 0 {
        Some(full)
    } else if padded {
        full.checked_add(4)
    } else {
        full.checked_add(rem + 1)
    }
}

/// Returns the maximum decoded length for an encoded input length.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_capacity;
///
/// assert_eq!(decoded_capacity(8), 6);
/// assert_eq!(decoded_capacity(7), 5);
/// ```
#[must_use]
pub const fn decoded_capacity(encoded_len: usize) -> usize {
    let rem = encoded_len % 4;
    encoded_len / 4 * 3
        + if rem == 2 {
            1
        } else if rem == 3 {
            2
        } else {
            0
        }
}

/// Returns the exact decoded length implied by input length and padding.
///
/// This validates padding placement and impossible lengths, but it does not
/// validate alphabet membership or non-canonical trailing bits.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_len;
///
/// assert_eq!(decoded_len(b"aGVsbG8=", true).unwrap(), 5);
/// assert_eq!(decoded_len(b"aGVsbG8", false).unwrap(), 5);
/// ```
pub fn decoded_len(input: &[u8], padded: bool) -> Result<usize, DecodeError> {
    if padded {
        decoded_len_padded(input)
    } else {
        decoded_len_unpadded(input)
    }
}

pub(crate) fn decoded_len_padded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let Some((&last, before_last_prefix)) = input.split_last() else {
        return Ok(0);
    };
    let Some(&before_last) = before_last_prefix.last() else {
        return Err(DecodeError::InvalidLength);
    };

    let mut padding = 0;
    if last == b'=' {
        padding += 1;
    }
    if before_last == b'=' {
        padding += 1;
    }
    if padding == 0
        && let Some(index) = input.iter().position(|byte| *byte == b'=')
    {
        return Err(DecodeError::InvalidPadding { index });
    }
    if padding > 0 {
        let first_pad = input.len() - padding;
        if let Some(index) = input[..first_pad].iter().position(|byte| *byte == b'=') {
            return Err(DecodeError::InvalidPadding { index });
        }
    }
    Ok(input.len() / 4 * 3 - padding)
}

pub(crate) fn decoded_len_unpadded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }
    if let Some(index) = input.iter().position(|byte| *byte == b'=') {
        return Err(DecodeError::InvalidPadding { index });
    }
    Ok(decoded_capacity(input.len()))
}
