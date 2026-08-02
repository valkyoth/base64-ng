use base64_ng::{OperationError, Status};

/// Cumulative input and output limits for one fragmented transform.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BytesLimits {
    max_input_len: usize,
    max_output_len: usize,
}

impl BytesLimits {
    /// No caller-selected input or output limit.
    pub const UNBOUNDED: Self = Self::new(usize::MAX, usize::MAX);

    /// Constructs cumulative input and output limits.
    #[must_use]
    pub const fn new(max_input_len: usize, max_output_len: usize) -> Self {
        Self {
            max_input_len,
            max_output_len,
        }
    }

    /// Returns the maximum input bytes accepted since reset.
    #[must_use]
    pub const fn max_input_len(self) -> usize {
        self.max_input_len
    }

    /// Returns the maximum output bytes committed since reset.
    #[must_use]
    pub const fn max_output_len(self) -> usize {
        self.max_output_len
    }
}

/// Exact prefix progress committed by one bytes-adapter call.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BytesProgress {
    input_consumed: usize,
    output_committed: usize,
}

impl BytesProgress {
    /// No input or output progress.
    pub const ZERO: Self = Self::new(0, 0);

    pub(crate) const fn new(input_consumed: usize, output_committed: usize) -> Self {
        Self {
            input_consumed,
            output_committed,
        }
    }

    /// Returns input bytes irrevocably advanced by this call.
    #[must_use]
    pub const fn input_consumed(self) -> usize {
        self.input_consumed
    }

    /// Returns output bytes passed to the destination by this call.
    #[must_use]
    pub const fn output_committed(self) -> usize {
        self.output_committed
    }

    pub(crate) fn add_input(&mut self, amount: usize) -> bool {
        let Some(value) = self.input_consumed.checked_add(amount) else {
            return false;
        };
        self.input_consumed = value;
        true
    }

    pub(crate) fn add_output(&mut self, amount: usize) -> bool {
        let Some(value) = self.output_committed.checked_add(amount) else {
            return false;
        };
        self.output_committed = value;
        true
    }
}

/// A successful stateful bytes-adapter call.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BytesStep {
    progress: BytesProgress,
    status: Status,
}

impl BytesStep {
    pub(crate) const fn new(progress: BytesProgress, status: Status) -> Self {
        Self { progress, status }
    }

    /// Returns exact progress committed by this call.
    #[must_use]
    pub const fn progress(self) -> BytesProgress {
        self.progress
    }

    /// Returns the shared 2.0 incremental-core status.
    #[must_use]
    pub const fn status(self) -> Status {
        self.status
    }
}

/// Stable classification for a bytes integration failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BytesErrorKind {
    /// Reported input would exceed the cumulative input limit.
    InputLimitExceeded {
        /// Required cumulative input bytes.
        required: usize,
        /// Configured maximum input bytes.
        limit: usize,
    },
    /// Producing the complete result would exceed the cumulative output limit.
    OutputLimitExceeded {
        /// Configured maximum output bytes.
        limit: usize,
    },
    /// Required length arithmetic overflowed `usize`.
    LengthOverflow,
    /// Reserving the crate-owned transactional destination failed.
    AllocationFailed {
        /// Requested destination capacity.
        requested: usize,
    },
    /// A custom [`bytes::Buf`] violated its safe trait contract.
    InvalidInputBuffer {
        /// Bytes the buffer still reported as remaining.
        remaining: usize,
    },
    /// The shared 2.0 incremental core rejected the operation.
    Operation(OperationError),
    /// A previous error or downstream panic permanently closed the adapter.
    FailedState,
    /// The companion observed an impossible core or buffer progress report.
    ImpossibleState,
}

/// Error with exact progress committed before failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BytesError {
    progress: BytesProgress,
    kind: BytesErrorKind,
}

impl BytesError {
    pub(crate) const fn new(progress: BytesProgress, kind: BytesErrorKind) -> Self {
        Self { progress, kind }
    }

    /// Returns exact input/output progress committed before the error.
    #[must_use]
    pub const fn progress(self) -> BytesProgress {
        self.progress
    }

    /// Returns the stable error classification.
    #[must_use]
    pub const fn kind(self) -> BytesErrorKind {
        self.kind
    }
}

impl core::fmt::Display for BytesError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            BytesErrorKind::InputLimitExceeded { required, limit } => {
                write!(
                    formatter,
                    "bytes input length {required} exceeds limit {limit}"
                )
            }
            BytesErrorKind::OutputLimitExceeded { limit } => {
                write!(formatter, "base64 output exceeds bytes limit {limit}")
            }
            BytesErrorKind::LengthOverflow => {
                formatter.write_str("base64 bytes length overflows usize")
            }
            BytesErrorKind::AllocationFailed { requested } => {
                write!(formatter, "failed to reserve {requested} bytes")
            }
            BytesErrorKind::InvalidInputBuffer { remaining } => write!(
                formatter,
                "Buf returned an empty or inconsistent chunk with {remaining} bytes remaining"
            ),
            BytesErrorKind::Operation(error) => error.fmt(formatter),
            BytesErrorKind::FailedState => {
                formatter.write_str("bytes transform is permanently failed")
            }
            BytesErrorKind::ImpossibleState => {
                formatter.write_str("bytes transform reached an impossible state")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            BytesErrorKind::Operation(error) => Some(error),
            _ => None,
        }
    }
}
