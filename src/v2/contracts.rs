//! Error, progress, lifecycle, and reporting contracts for the 2.0 core.

use core::num::NonZeroUsize;

#[path = "contracts/reporting.rs"]
mod reporting;

// These become reachable when the complete 2.0 surface is exposed. Commit 8
// keeps the model private while compiling its future external shape in CI.
#[allow(unused_imports)]
pub(crate) use super::lifecycle::Lifecycle;
#[allow(unused_imports)]
pub use super::lifecycle::SourceSpan;
#[allow(unused_imports)]
pub use reporting::{AssuranceClass, Atomicity, BackendClass, ProtocolScope};

/// Exact progress committed by one transform call.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Progress {
    input_consumed: usize,
    output_produced: usize,
}

impl Progress {
    /// A call that consumed and produced no bytes.
    pub const ZERO: Self = Self::new(0, 0);

    /// Constructs an exact progress report for crate-owned state machines.
    pub(crate) const fn new(input_consumed: usize, output_produced: usize) -> Self {
        Self {
            input_consumed,
            output_produced,
        }
    }

    /// Returns the input prefix accepted by this call.
    #[must_use]
    pub const fn input_consumed(self) -> usize {
        self.input_consumed
    }

    /// Returns the output prefix initialized by this call.
    #[must_use]
    pub const fn output_produced(self) -> usize {
        self.output_produced
    }
}

/// Retry information when the current destination cannot make progress.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OutputFull {
    minimum_output: NonZeroUsize,
}

impl OutputFull {
    /// Constructs a retry requirement that always permits progress.
    pub(crate) const fn new(minimum_output: NonZeroUsize) -> Self {
        Self { minimum_output }
    }

    /// Returns the minimum destination bytes needed by the next retry.
    #[must_use]
    pub const fn minimum_output(self) -> NonZeroUsize {
        self.minimum_output
    }
}

/// Non-failing state reached after one transform call.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Status {
    /// More input or an explicit finish call is required.
    NeedInput,
    /// Retry with at least the reported output capacity.
    OutputFull(OutputFull),
    /// The transform completed successfully and accepts no more input.
    Complete,
}

impl Status {
    /// Returns the stable lowercase identifier for this status class.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NeedInput => "need-input",
            Self::OutputFull(_) => "output-full",
            Self::Complete => "complete",
        }
    }
}

/// One non-failing transform result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Step {
    progress: Progress,
    status: Status,
}

impl Step {
    pub(crate) const fn new(progress: Progress, status: Status) -> Self {
        Self { progress, status }
    }

    /// Returns exact input and output progress for this call.
    #[must_use]
    pub const fn progress(self) -> Progress {
        self.progress
    }

    /// Returns the state reached by this call.
    #[must_use]
    pub const fn status(self) -> Status {
        self.status
    }
}

/// Redacted malformed-input classification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum InputErrorKind {
    /// A byte is outside the selected alphabet.
    InvalidByte,
    /// Padding is missing, forbidden, misplaced, or excessive.
    InvalidPadding,
    /// Unused trailing bits are non-zero.
    NonCanonicalTrailingBits,
    /// The encoded length cannot represent a complete value.
    InvalidLength,
    /// Finishing found an incomplete encoded quantum.
    TruncatedInput,
    /// Input followed a terminal padded quantum.
    TrailingData,
    /// Wrapped body layout is malformed.
    InvalidLineWrap,
}

impl InputErrorKind {
    /// Returns the stable lowercase identifier for this error class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidByte => "invalid-byte",
            Self::InvalidPadding => "invalid-padding",
            Self::NonCanonicalTrailingBits => "noncanonical-trailing-bits",
            Self::InvalidLength => "invalid-length",
            Self::TruncatedInput => "truncated-input",
            Self::TrailingData => "trailing-data",
            Self::InvalidLineWrap => "invalid-line-wrap",
        }
    }
}

/// Detailed ordinary malformed-input diagnostic.
///
/// `Debug` is redacted. `Display` includes ordinary input diagnostics and must
/// not be logged for secret-bearing input.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum InputError {
    /// A byte is outside the selected alphabet.
    InvalidByte {
        /// Original absolute source index.
        index: usize,
        /// Rejected byte.
        byte: u8,
    },
    /// Padding became invalid at this original absolute source index.
    InvalidPadding {
        /// Original absolute source index.
        index: usize,
    },
    /// Unused trailing bits are non-zero.
    NonCanonicalTrailingBits {
        /// Original absolute source index of the final significant symbol.
        index: usize,
    },
    /// The encoded length cannot represent a complete value.
    InvalidLength,
    /// Finishing found an incomplete encoded quantum.
    TruncatedInput {
        /// Original absolute source index at end of input.
        index: usize,
    },
    /// Input followed a terminal padded quantum.
    TrailingData {
        /// Original absolute source index of the first trailing byte.
        index: usize,
    },
    /// Wrapped body layout is malformed.
    InvalidLineWrap {
        /// Original absolute source index where layout became invalid.
        index: usize,
    },
}

impl InputError {
    /// Returns a redacted, stable error class.
    #[must_use]
    pub const fn kind(self) -> InputErrorKind {
        match self {
            Self::InvalidByte { .. } => InputErrorKind::InvalidByte,
            Self::InvalidPadding { .. } => InputErrorKind::InvalidPadding,
            Self::NonCanonicalTrailingBits { .. } => InputErrorKind::NonCanonicalTrailingBits,
            Self::InvalidLength => InputErrorKind::InvalidLength,
            Self::TruncatedInput { .. } => InputErrorKind::TruncatedInput,
            Self::TrailingData { .. } => InputErrorKind::TrailingData,
            Self::InvalidLineWrap { .. } => InputErrorKind::InvalidLineWrap,
        }
    }
}

impl core::fmt::Debug for InputError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("InputError")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

impl core::fmt::Display for InputError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidByte { index, byte } => {
                write!(
                    formatter,
                    "invalid byte 0x{byte:02x} at source index {index}"
                )
            }
            Self::InvalidPadding { index } => {
                write!(formatter, "invalid padding at source index {index}")
            }
            Self::NonCanonicalTrailingBits { index } => {
                write!(
                    formatter,
                    "noncanonical trailing bits at source index {index}"
                )
            }
            Self::InvalidLength => formatter.write_str("invalid encoded input length"),
            Self::TruncatedInput { index } => {
                write!(formatter, "truncated input at source index {index}")
            }
            Self::TrailingData { index } => {
                write!(formatter, "trailing data at source index {index}")
            }
            Self::InvalidLineWrap { index } => {
                write!(formatter, "invalid line wrapping at source index {index}")
            }
        }
    }
}

/// Internal backend integrity failure, separate from attacker input errors.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BackendFault {
    /// A backend known-answer test failed.
    SelfTestFailed,
    /// Checked output diverged from the independent reference path.
    OutputMismatch,
    /// A backend reached an impossible internal state.
    ImpossibleState,
    /// Scalar retry after quarantining an accelerated backend failed.
    ScalarRetryFailed,
}

impl BackendFault {
    /// Returns the stable lowercase identifier for this fault class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SelfTestFailed => "backend-self-test-failed",
            Self::OutputMismatch => "backend-output-mismatch",
            Self::ImpossibleState => "backend-impossible-state",
            Self::ScalarRetryFailed => "backend-scalar-retry-failed",
        }
    }
}

/// Absorbing transform failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Failure {
    /// Attacker-controlled or otherwise malformed ordinary input.
    Input(InputError),
    /// Absolute source position can no longer be represented by `usize`.
    PositionOverflow,
    /// Internal backend integrity failure.
    Backend(BackendFault),
    /// A caller-declared input, output, allocation, or frame limit was exceeded.
    ResourceLimit,
}

impl Failure {
    /// Returns the stable lowercase identifier for this failure class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input(error) => error.kind().as_str(),
            Self::PositionOverflow => "position-overflow",
            Self::Backend(fault) => fault.as_str(),
            Self::ResourceLimit => "resource-limit",
        }
    }
}

/// Illegal call against a successfully completed state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TerminalError {
    /// New input was supplied after finalization began but before output drained.
    InputAfterFinish,
    /// New input was supplied after successful completion.
    InputAfterComplete,
}

impl TerminalError {
    /// Returns the stable lowercase identifier for this terminal call error.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InputAfterFinish => "input-after-finish",
            Self::InputAfterComplete => "input-after-complete",
        }
    }
}

/// Error returned by a lifecycle operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum OperationError {
    /// The transform entered or was already in an absorbing failure state.
    Failed(Failure),
    /// The call is not legal after successful completion.
    Terminal(TerminalError),
}

impl OperationError {
    /// Returns the stable lowercase identifier for this error class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Failed(failure) => failure.as_str(),
            Self::Terminal(error) => error.as_str(),
        }
    }
}

impl core::fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Failed(Failure::Input(error)) => error.fmt(formatter),
            Self::Failed(failure) => formatter.write_str(failure.as_str()),
            Self::Terminal(error) => formatter.write_str(error.as_str()),
        }
    }
}

impl core::error::Error for OperationError {}
