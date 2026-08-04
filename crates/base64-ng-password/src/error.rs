/// Stable password-record failure class.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PasswordRecordErrorKind {
    /// The modular-crypt identifier is unsupported or malformed.
    InvalidPrefix,
    /// The record has missing, extra, or misplaced fields.
    InvalidStructure,
    /// The rounds field is out of range or noncanonical.
    InvalidRounds,
    /// A standalone adapted-Base64 field is malformed or noncanonical.
    InvalidField,
    /// The salt field is malformed, noncanonical, or too long for the format.
    InvalidSalt,
    /// The checksum field is malformed, noncanonical, or has the wrong length.
    InvalidChecksum,
    /// Source record or field input exceeded its finite limit.
    InputLimitExceeded,
    /// One encoded field exceeded its finite limit.
    FieldLimitExceeded,
    /// Decoded output exceeded its finite limit.
    DecodedOutputLimitExceeded,
    /// Generated output exceeded its finite limit.
    OutputLimitExceeded,
    /// Cumulative input-byte work before completion exceeded its finite limit.
    WorkLimitExceeded,
    /// Caller-owned output storage was too small.
    OutputTooSmall,
    /// Length or position arithmetic overflowed `usize`.
    LengthOverflow,
    /// Allocation could not be reserved.
    AllocationFailed,
    /// The shared Base64 backend reported an integrity failure.
    BackendFailure,
}

/// Password-record failure with bounded, content-free diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PasswordRecordError {
    kind: PasswordRecordErrorKind,
    position: Option<usize>,
    required: Option<usize>,
    available: Option<usize>,
}

impl PasswordRecordError {
    pub(crate) const fn new(kind: PasswordRecordErrorKind) -> Self {
        Self {
            kind,
            position: None,
            required: None,
            available: None,
        }
    }

    pub(crate) const fn at(kind: PasswordRecordErrorKind, position: usize) -> Self {
        let mut error = Self::new(kind);
        error.position = Some(position);
        error
    }

    pub(crate) const fn capacity(required: usize, available: usize) -> Self {
        let mut error = Self::new(PasswordRecordErrorKind::OutputTooSmall);
        error.required = Some(required);
        error.available = Some(available);
        error
    }

    /// Returns the stable error class.
    #[must_use]
    pub const fn kind(self) -> PasswordRecordErrorKind {
        self.kind
    }

    /// Returns a source position without returning source content.
    #[must_use]
    pub const fn position(self) -> Option<usize> {
        self.position
    }

    /// Returns the exact required output size when available.
    #[must_use]
    pub const fn required(self) -> Option<usize> {
        self.required
    }

    /// Returns the caller-provided output size when available.
    #[must_use]
    pub const fn available(self) -> Option<usize> {
        self.available
    }
}

impl core::fmt::Display for PasswordRecordError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            PasswordRecordErrorKind::OutputTooSmall => write!(
                formatter,
                "password-record output too small: required {}, available {}",
                self.required.unwrap_or(0),
                self.available.unwrap_or(0)
            ),
            _ => write!(
                formatter,
                "password-record transform failed: {:?}",
                self.kind
            ),
        }
    }
}

pub(crate) fn map_base64(
    error: base64_ng::OneShotError,
    invalid: PasswordRecordErrorKind,
) -> PasswordRecordError {
    match error {
        base64_ng::OneShotError::LengthOverflow | base64_ng::OneShotError::PositionOverflow => {
            PasswordRecordError::new(PasswordRecordErrorKind::LengthOverflow)
        }
        base64_ng::OneShotError::Input(input) => {
            let position = match input {
                base64_ng::InputError::InvalidByte { index, .. }
                | base64_ng::InputError::InvalidPadding { index }
                | base64_ng::InputError::NonCanonicalTrailingBits { index }
                | base64_ng::InputError::TruncatedInput { index }
                | base64_ng::InputError::TrailingData { index }
                | base64_ng::InputError::InvalidLineWrap { index } => Some(index),
                _ => None,
            };
            position.map_or_else(
                || PasswordRecordError::new(invalid),
                |index| PasswordRecordError::at(invalid, index),
            )
        }
        base64_ng::OneShotError::OutputTooSmall {
            required,
            available,
        } => PasswordRecordError::capacity(required, available),
        base64_ng::OneShotError::AllocationLimitExceeded { .. } => {
            PasswordRecordError::new(PasswordRecordErrorKind::OutputLimitExceeded)
        }
        base64_ng::OneShotError::AllocationFailed { .. } => {
            PasswordRecordError::new(PasswordRecordErrorKind::AllocationFailed)
        }
        _ => PasswordRecordError::new(PasswordRecordErrorKind::BackendFailure),
    }
}
