use crate::PemLabelError;

/// Classified RFC 7468 operation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PemErrorKind {
    /// Source input exceeded its finite limit.
    InputLimitExceeded,
    /// Generated textual output exceeded its finite limit.
    EncodedOutputLimitExceeded,
    /// Decoded payload output exceeded its finite limit.
    DecodedOutputLimitExceeded,
    /// A physical line exceeded its finite limit.
    PhysicalLineTooLong,
    /// A label exceeded its finite limit.
    LabelLimitExceeded,
    /// The document contained too many blocks.
    BlockLimitExceeded,
    /// Surrounding text exceeded its finite limit.
    AdjacentTextLimitExceeded,
    /// Work before output exceeded its finite limit.
    WorkLimitExceeded,
    /// No BEGIN boundary was found.
    BeginBoundaryMissing,
    /// A boundary was malformed.
    InvalidBoundary,
    /// A label violated RFC 7468 syntax.
    InvalidLabel,
    /// Canonical generation or strict parsing rejected lowercase label text.
    NonCanonicalLabel,
    /// BEGIN and END labels did not match.
    MismatchedEndLabel,
    /// Legacy encapsulated headers are outside RFC 7468 scope.
    LegacyHeadersNotSupported,
    /// The Base64 body was malformed or noncanonical.
    InvalidBody,
    /// Strict Figure 3 line layout was violated.
    NonCanonicalLayout,
    /// An END boundary was missing.
    MissingEndBoundary,
    /// Caller-owned output storage was too small.
    OutputTooSmall,
    /// Length arithmetic overflowed `usize`.
    LengthOverflow,
    /// Allocation could not be reserved.
    AllocationFailed,
    /// An incremental parser was already terminal.
    TerminalState,
    /// A secret operation requires exactly one matching block.
    SecretBlockSelection,
    /// An internal validated-output invariant did not hold.
    InternalInvariantViolation,
}

/// RFC 7468 error with an optional public source position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PemError {
    kind: PemErrorKind,
    position: Option<usize>,
    required: Option<usize>,
    available: Option<usize>,
}

impl PemError {
    pub(crate) const fn new(kind: PemErrorKind) -> Self {
        Self {
            kind,
            position: None,
            required: None,
            available: None,
        }
    }

    pub(crate) const fn at(kind: PemErrorKind, position: usize) -> Self {
        Self {
            kind,
            position: Some(position),
            required: None,
            available: None,
        }
    }

    pub(crate) const fn capacity(required: usize, available: usize) -> Self {
        Self {
            kind: PemErrorKind::OutputTooSmall,
            position: None,
            required: Some(required),
            available: Some(available),
        }
    }

    /// Returns the stable error class.
    #[must_use]
    pub const fn kind(self) -> PemErrorKind {
        self.kind
    }

    /// Returns a public source position when available.
    #[must_use]
    pub const fn position(self) -> Option<usize> {
        self.position
    }
}

impl From<PemLabelError> for PemError {
    fn from(error: PemLabelError) -> Self {
        match error {
            PemLabelError::InvalidSyntax => Self::new(PemErrorKind::InvalidLabel),
            PemLabelError::AllocationFailed => Self::new(PemErrorKind::AllocationFailed),
        }
    }
}

impl core::fmt::Display for PemError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let (Some(required), Some(available)) = (self.required, self.available) {
            return write!(
                formatter,
                "PEM output buffer too small: required {required}, available {available}"
            );
        }
        write!(formatter, "PEM operation failed: {:?}", self.kind)?;
        if let Some(position) = self.position {
            write!(formatter, " at byte {position}")?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PemError {}

pub(crate) fn map_base64(_: base64_ng::OneShotError) -> PemError {
    PemError::new(PemErrorKind::InvalidBody)
}
