use base64_ng::{Failure, InputError, OneShotError, OperationError};

/// Stable modified-Base64 payload failure class.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ImapPayloadErrorKind {
    /// Payload bytes were outside the alphabet, padded, or noncanonical.
    InvalidPayload,
    /// Decoded UTF-16BE storage had an odd byte length.
    InvalidUtf16BeLength,
    /// Source input exceeded its finite limit.
    InputLimitExceeded,
    /// Destination output exceeded its finite limit.
    OutputLimitExceeded,
    /// Work before completion exceeded its finite limit.
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

/// Modified-Base64 payload failure with bounded ordinary diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImapPayloadError {
    kind: ImapPayloadErrorKind,
    position: Option<usize>,
    required: Option<usize>,
    available: Option<usize>,
}

impl ImapPayloadError {
    pub(crate) const fn new(kind: ImapPayloadErrorKind) -> Self {
        Self {
            kind,
            position: None,
            required: None,
            available: None,
        }
    }

    pub(crate) const fn capacity(required: usize, available: usize) -> Self {
        let mut error = Self::new(ImapPayloadErrorKind::OutputTooSmall);
        error.required = Some(required);
        error.available = Some(available);
        error
    }

    /// Returns the stable error class.
    #[must_use]
    pub const fn kind(self) -> ImapPayloadErrorKind {
        self.kind
    }

    /// Returns an ordinary source byte position when available.
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

impl core::fmt::Display for ImapPayloadError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            ImapPayloadErrorKind::OutputTooSmall => write!(
                formatter,
                "IMAP payload output too small: required {}, available {}",
                self.required.unwrap_or(0),
                self.available.unwrap_or(0)
            ),
            _ => write!(
                formatter,
                "IMAP modified-Base64 payload operation failed: {:?}",
                self.kind
            ),
        }
    }
}

pub(crate) fn map_one_shot(error: OneShotError) -> ImapPayloadError {
    match error {
        OneShotError::LengthOverflow | OneShotError::PositionOverflow => {
            ImapPayloadError::new(ImapPayloadErrorKind::LengthOverflow)
        }
        OneShotError::Input(input) => map_input(input),
        OneShotError::OutputTooSmall {
            required,
            available,
        } => ImapPayloadError::capacity(required, available),
        OneShotError::AllocationLimitExceeded { .. } => {
            ImapPayloadError::new(ImapPayloadErrorKind::OutputLimitExceeded)
        }
        OneShotError::AllocationFailed { .. } => {
            ImapPayloadError::new(ImapPayloadErrorKind::AllocationFailed)
        }
        _ => ImapPayloadError::new(ImapPayloadErrorKind::BackendFailure),
    }
}

pub(crate) fn map_operation(error: OperationError) -> ImapPayloadError {
    match error {
        OperationError::Failed(Failure::Input(input)) => map_input(input),
        OperationError::Failed(Failure::PositionOverflow) => {
            ImapPayloadError::new(ImapPayloadErrorKind::LengthOverflow)
        }
        OperationError::Failed(Failure::ResourceLimit) => {
            ImapPayloadError::new(ImapPayloadErrorKind::OutputLimitExceeded)
        }
        OperationError::Failed(Failure::Backend(_)) => {
            ImapPayloadError::new(ImapPayloadErrorKind::BackendFailure)
        }
        OperationError::Terminal(_) => ImapPayloadError::new(ImapPayloadErrorKind::TerminalState),
        _ => ImapPayloadError::new(ImapPayloadErrorKind::BackendFailure),
    }
}

fn map_input(error: InputError) -> ImapPayloadError {
    let position = match error {
        InputError::InvalidByte { index, .. }
        | InputError::InvalidPadding { index }
        | InputError::NonCanonicalTrailingBits { index }
        | InputError::TruncatedInput { index }
        | InputError::TrailingData { index }
        | InputError::InvalidLineWrap { index } => Some(index),
        _ => None,
    };
    let mut mapped = ImapPayloadError::new(ImapPayloadErrorKind::InvalidPayload);
    mapped.position = position;
    mapped
}
