use base64_ng::OneShotError;

/// Stable classification for an RFC 2045 body failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MimeBodyErrorKind {
    /// Source input exceeded the selected finite bound.
    InputLimitExceeded,
    /// Encoded or decoded output exceeded the selected finite bound.
    OutputLimitExceeded,
    /// One physical transport line exceeded the selected finite bound.
    PhysicalLineTooLong,
    /// Too many bytes outside RFC 2045 Table 1 were ignored.
    SkippedNonalphabetLimitExceeded,
    /// Too much source input was processed without producing a quantum.
    WorkBeforeOutputLimitExceeded,
    /// Strict canonical MIME body layout was malformed.
    InvalidCanonicalLayout,
    /// Base64 symbols, padding, length, or trailing bits were malformed.
    InvalidBase64,
    /// The caller-owned destination was too small.
    OutputTooSmall,
    /// Length arithmetic overflowed.
    LengthOverflow,
    /// Allocation failed for an allocating helper.
    AllocationFailed,
    /// The state was used after completion or failure.
    TerminalState,
    /// An internal invariant failed.
    InternalInvariant,
}

/// Detailed ordinary error for RFC 2045 Base64 content-transfer bodies.
///
/// `Debug` exposes only the stable class. `Display` may include a public input
/// position and must not be logged for secret-bearing input. This companion
/// has no secret-processing claim.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MimeBodyError {
    kind: MimeBodyErrorKind,
    index: Option<usize>,
    required: Option<usize>,
    available: Option<usize>,
}

impl MimeBodyError {
    pub(crate) const fn new(kind: MimeBodyErrorKind) -> Self {
        Self {
            kind,
            index: None,
            required: None,
            available: None,
        }
    }

    pub(crate) const fn at(kind: MimeBodyErrorKind, index: usize) -> Self {
        Self {
            kind,
            index: Some(index),
            required: None,
            available: None,
        }
    }

    pub(crate) const fn capacity(required: usize, available: usize) -> Self {
        Self {
            kind: MimeBodyErrorKind::OutputTooSmall,
            index: None,
            required: Some(required),
            available: Some(available),
        }
    }

    /// Returns the stable redacted error class.
    #[must_use]
    pub const fn kind(self) -> MimeBodyErrorKind {
        self.kind
    }

    /// Returns the ordinary source index when one is available.
    #[must_use]
    pub const fn source_index(self) -> Option<usize> {
        self.index
    }

    /// Returns the exact required output size when known.
    #[must_use]
    pub const fn required_output(self) -> Option<usize> {
        self.required
    }

    /// Returns the available destination size when known.
    #[must_use]
    pub const fn available_output(self) -> Option<usize> {
        self.available
    }
}

impl core::fmt::Debug for MimeBodyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("MimeBodyError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl core::fmt::Display for MimeBodyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let (Some(required), Some(available)) = (self.required, self.available) {
            return write!(
                formatter,
                "MIME body output requires {required} bytes; {available} available"
            );
        }
        if let Some(index) = self.index {
            return write!(
                formatter,
                "MIME body {} at source index {index}",
                self.kind.as_str()
            );
        }
        write!(formatter, "MIME body {}", self.kind.as_str())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MimeBodyError {}

impl MimeBodyErrorKind {
    /// Returns the stable lowercase error identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InputLimitExceeded => "input-limit-exceeded",
            Self::OutputLimitExceeded => "output-limit-exceeded",
            Self::PhysicalLineTooLong => "physical-line-too-long",
            Self::SkippedNonalphabetLimitExceeded => "skipped-nonalphabet-limit-exceeded",
            Self::WorkBeforeOutputLimitExceeded => "work-before-output-limit-exceeded",
            Self::InvalidCanonicalLayout => "invalid-canonical-layout",
            Self::InvalidBase64 => "invalid-base64",
            Self::OutputTooSmall => "output-too-small",
            Self::LengthOverflow => "length-overflow",
            Self::AllocationFailed => "allocation-failed",
            Self::TerminalState => "terminal-state",
            Self::InternalInvariant => "internal-invariant",
        }
    }
}

pub(crate) const fn map_base64(error: OneShotError) -> MimeBodyError {
    match error {
        OneShotError::LengthOverflow | OneShotError::PositionOverflow => {
            MimeBodyError::new(MimeBodyErrorKind::LengthOverflow)
        }
        OneShotError::Input(_) => MimeBodyError::new(MimeBodyErrorKind::InvalidBase64),
        OneShotError::OutputTooSmall {
            required,
            available,
        } => MimeBodyError::capacity(required, available),
        OneShotError::AllocationLimitExceeded { .. } => {
            MimeBodyError::new(MimeBodyErrorKind::OutputLimitExceeded)
        }
        OneShotError::AllocationFailed { .. } => {
            MimeBodyError::new(MimeBodyErrorKind::AllocationFailed)
        }
        _ => MimeBodyError::new(MimeBodyErrorKind::InternalInvariant),
    }
}
