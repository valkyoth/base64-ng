use core::num::NonZeroUsize;

/// Non-failing incremental state after one operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImapPayloadStatus {
    /// More input or an explicit finish call is required.
    NeedInput,
    /// Retry with at least the reported destination capacity.
    OutputFull(NonZeroUsize),
    /// The complete payload was emitted and the state is terminal.
    Complete,
}

/// Exact incremental progress for one call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImapPayloadStep {
    input_consumed: usize,
    output_produced: usize,
    status: ImapPayloadStatus,
}

impl ImapPayloadStep {
    pub(crate) const fn new(
        input_consumed: usize,
        output_produced: usize,
        status: ImapPayloadStatus,
    ) -> Self {
        Self {
            input_consumed,
            output_produced,
            status,
        }
    }

    /// Returns source bytes accepted by this call.
    #[must_use]
    pub const fn input_consumed(self) -> usize {
        self.input_consumed
    }

    /// Returns destination bytes initialized by this call.
    #[must_use]
    pub const fn output_produced(self) -> usize {
        self.output_produced
    }

    /// Returns the resulting incremental state.
    #[must_use]
    pub const fn status(self) -> ImapPayloadStatus {
        self.status
    }
}
