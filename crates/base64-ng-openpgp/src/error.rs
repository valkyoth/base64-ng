/// Classified `OpenPGP` armor operation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OpenPgpErrorKind {
    /// Source input exceeded its finite limit.
    InputLimitExceeded,
    /// Generated textual output exceeded its finite limit.
    EncodedOutputLimitExceeded,
    /// Decoded payload output exceeded its finite limit.
    DecodedOutputLimitExceeded,
    /// A physical line exceeded its finite limit.
    PhysicalLineTooLong,
    /// A boundary label exceeded its finite limit.
    LabelLimitExceeded,
    /// The document contained too many blocks.
    BlockLimitExceeded,
    /// Adjacent document whitespace exceeded its finite limit.
    AdjacentDocumentLimitExceeded,
    /// The header count exceeded its finite limit.
    HeaderCountLimitExceeded,
    /// Retained header bytes exceeded their finite limit.
    HeaderBytesLimitExceeded,
    /// Work before output exceeded its finite limit.
    WorkLimitExceeded,
    /// No armor block was present.
    BeginBoundaryMissing,
    /// An armor boundary or type was malformed or unsupported.
    InvalidBoundary,
    /// BEGIN and END armor types did not match.
    MismatchedEndBoundary,
    /// An armor header was malformed.
    InvalidHeader,
    /// The blank separator after armor headers was missing.
    MissingHeaderSeparator,
    /// The Base64 body was malformed or noncanonical.
    InvalidBody,
    /// A Base64 body line exceeded RFC 9580's 76-character limit.
    BodyLineTooLong,
    /// A required CRC-24 footer was absent.
    ChecksumMissing,
    /// A CRC-24 footer was malformed.
    ChecksumMalformed,
    /// A CRC-24 footer did not match the decoded payload.
    ChecksumMismatch,
    /// A closing boundary was missing.
    MissingEndBoundary,
    /// Non-whitespace data appeared outside an armor block.
    TrailingAmbiguity,
    /// Caller-owned output storage was too small.
    OutputTooSmall,
    /// Length arithmetic overflowed `usize`.
    LengthOverflow,
    /// Allocation could not be reserved.
    AllocationFailed,
    /// An incremental object was already terminal.
    TerminalState,
    /// A secret operation did not select exactly one expected block.
    SecretBlockSelection,
    /// A standard I/O operation failed.
    Io,
    /// An internal validated-output invariant failed.
    InternalInvariantViolation,
}

/// Content-free `OpenPGP` armor error with an optional public byte position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenPgpError {
    kind: OpenPgpErrorKind,
    position: Option<usize>,
    required: Option<usize>,
    available: Option<usize>,
}

impl OpenPgpError {
    pub(crate) const fn new(kind: OpenPgpErrorKind) -> Self {
        Self {
            kind,
            position: None,
            required: None,
            available: None,
        }
    }

    pub(crate) const fn at(kind: OpenPgpErrorKind, position: usize) -> Self {
        Self {
            kind,
            position: Some(position),
            required: None,
            available: None,
        }
    }

    pub(crate) const fn capacity(required: usize, available: usize) -> Self {
        Self {
            kind: OpenPgpErrorKind::OutputTooSmall,
            position: None,
            required: Some(required),
            available: Some(available),
        }
    }

    /// Returns the stable error class.
    #[must_use]
    pub const fn kind(self) -> OpenPgpErrorKind {
        self.kind
    }

    /// Returns a public source byte position when available.
    #[must_use]
    pub const fn position(self) -> Option<usize> {
        self.position
    }
}

impl core::fmt::Display for OpenPgpError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let (Some(required), Some(available)) = (self.required, self.available) {
            return write!(
                formatter,
                "OpenPGP armor output too small: required {required}, available {available}"
            );
        }
        write!(formatter, "OpenPGP armor operation failed: {:?}", self.kind)?;
        if let Some(position) = self.position {
            write!(formatter, " at byte {position}")?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OpenPgpError {}

pub(crate) fn map_base64(_: base64_ng::OneShotError) -> OpenPgpError {
    OpenPgpError::new(OpenPgpErrorKind::InvalidBody)
}
