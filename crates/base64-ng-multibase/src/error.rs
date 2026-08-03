use base64_ng::{Failure, InputError, OneShotError, OperationError};

/// Stable Base64-family multibase failure class.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Base64MultibaseErrorKind {
    /// The input had no prefix byte.
    MissingPrefix,
    /// The prefix is not one of the four admitted Base64-family entries.
    UnsupportedPrefix,
    /// The prefix-selected Base64 payload was not strict and canonical.
    InvalidPayload,
    /// Source input exceeded its finite limit.
    InputLimitExceeded,
    /// Destination output exceeded its finite limit.
    OutputLimitExceeded,
    /// Work before complete output exceeded its finite limit.
    WorkLimitExceeded,
    /// Caller-owned output storage was too small.
    OutputTooSmall,
    /// Length or position arithmetic overflowed `usize`.
    LengthOverflow,
    /// Allocation could not be reserved.
    AllocationFailed,
    /// An incremental operation was called after completion or failure.
    TerminalState,
    /// The shared Base64 backend reported an integrity failure.
    BackendFailure,
}

/// Base64-family multibase failure with bounded ordinary diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Base64MultibaseError {
    kind: Base64MultibaseErrorKind,
    position: Option<usize>,
    prefix: Option<u8>,
    required: Option<usize>,
    available: Option<usize>,
}

impl Base64MultibaseError {
    pub(crate) const fn new(kind: Base64MultibaseErrorKind) -> Self {
        Self {
            kind,
            position: None,
            prefix: None,
            required: None,
            available: None,
        }
    }

    pub(crate) const fn unsupported(prefix: u8) -> Self {
        let mut error = Self::new(Base64MultibaseErrorKind::UnsupportedPrefix);
        error.position = Some(0);
        error.prefix = Some(prefix);
        error
    }

    pub(crate) const fn capacity(required: usize, available: usize) -> Self {
        let mut error = Self::new(Base64MultibaseErrorKind::OutputTooSmall);
        error.required = Some(required);
        error.available = Some(available);
        error
    }

    /// Returns the stable error class.
    #[must_use]
    pub const fn kind(self) -> Base64MultibaseErrorKind {
        self.kind
    }

    /// Returns an ordinary full-input byte position when available.
    #[must_use]
    pub const fn position(self) -> Option<usize> {
        self.position
    }

    /// Returns the rejected prefix for [`Base64MultibaseErrorKind::UnsupportedPrefix`].
    #[must_use]
    pub const fn prefix(self) -> Option<u8> {
        self.prefix
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

impl core::fmt::Display for Base64MultibaseError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            Base64MultibaseErrorKind::UnsupportedPrefix => {
                write!(
                    formatter,
                    "unsupported multibase prefix 0x{:02x}",
                    self.prefix.unwrap_or(0)
                )
            }
            Base64MultibaseErrorKind::OutputTooSmall => write!(
                formatter,
                "multibase output too small: required {}, available {}",
                self.required.unwrap_or(0),
                self.available.unwrap_or(0)
            ),
            _ => write!(
                formatter,
                "Base64-family multibase operation failed: {:?}",
                self.kind
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Base64MultibaseError {}

pub(crate) fn map_one_shot(error: OneShotError) -> Base64MultibaseError {
    match error {
        OneShotError::LengthOverflow | OneShotError::PositionOverflow => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::LengthOverflow)
        }
        OneShotError::Input(input) => map_input(input),
        OneShotError::OutputTooSmall {
            required,
            available,
        } => Base64MultibaseError::capacity(required, available),
        OneShotError::AllocationLimitExceeded { .. } => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::OutputLimitExceeded)
        }
        OneShotError::AllocationFailed { .. } => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::AllocationFailed)
        }
        OneShotError::Backend(_) => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::BackendFailure)
        }
        _ => Base64MultibaseError::new(Base64MultibaseErrorKind::BackendFailure),
    }
}

pub(crate) fn map_operation(error: OperationError) -> Base64MultibaseError {
    match error {
        OperationError::Failed(Failure::Input(input)) => map_input(input),
        OperationError::Failed(Failure::PositionOverflow) => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::LengthOverflow)
        }
        OperationError::Failed(Failure::ResourceLimit) => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::OutputLimitExceeded)
        }
        OperationError::Failed(Failure::Backend(_)) => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::BackendFailure)
        }
        OperationError::Terminal(_) => {
            Base64MultibaseError::new(Base64MultibaseErrorKind::TerminalState)
        }
        _ => Base64MultibaseError::new(Base64MultibaseErrorKind::BackendFailure),
    }
}

fn map_input(error: InputError) -> Base64MultibaseError {
    let payload_position = match error {
        InputError::InvalidByte { index, .. }
        | InputError::InvalidPadding { index }
        | InputError::NonCanonicalTrailingBits { index }
        | InputError::TruncatedInput { index }
        | InputError::TrailingData { index }
        | InputError::InvalidLineWrap { index } => Some(index),
        _ => None,
    };
    let mut mapped = Base64MultibaseError::new(Base64MultibaseErrorKind::InvalidPayload);
    mapped.position = payload_position.and_then(|position| position.checked_add(1));
    mapped
}
