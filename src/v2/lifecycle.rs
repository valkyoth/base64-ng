//! Shared source-position and finalization lifecycle for incremental states.

use core::num::NonZeroUsize;

use super::contracts::{
    BackendFault, Failure, OperationError, OutputFull, Progress, Status, Step, TerminalError,
};

/// Transactionally reserved absolute source range for one input chunk.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceSpan {
    start: usize,
    len: usize,
}

impl SourceSpan {
    /// Returns the original absolute index for a local source-byte offset.
    pub const fn index(self, local: usize) -> Option<usize> {
        if local >= self.len {
            return None;
        }
        self.start.checked_add(local)
    }

    /// Returns the number of original source bytes in this span.
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns whether the span contains no source bytes.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
struct SourcePosition {
    next: usize,
}

impl SourcePosition {
    const fn span(self, len: usize) -> Option<SourceSpan> {
        let start = self.next;
        let Some(_end) = start.checked_add(len) else {
            return None;
        };
        Some(SourceSpan { start, len })
    }

    const fn commit(&mut self, span: SourceSpan, consumed: usize) -> bool {
        if span.start != self.next || consumed > span.len {
            return false;
        }
        let Some(next) = self.next.checked_add(consumed) else {
            return false;
        };
        self.next = next;
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Phase {
    Active,
    Finishing,
    Complete,
    Failed(Failure),
}

/// Shared lifecycle model consumed by incremental state machines.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Lifecycle {
    phase: Phase,
    source: SourcePosition,
}

impl Lifecycle {
    pub(crate) const fn new() -> Self {
        Self {
            phase: Phase::Active,
            source: SourcePosition { next: 0 },
        }
    }

    pub(crate) fn reserve_input(&mut self, len: usize) -> Result<SourceSpan, OperationError> {
        self.require_active()?;
        match self.source.span(len) {
            Some(span) => Ok(span),
            None => Err(self.latch(Failure::PositionOverflow)),
        }
    }

    pub(crate) fn commit_input(
        &mut self,
        span: SourceSpan,
        consumed: usize,
    ) -> Result<(), OperationError> {
        self.require_active()?;
        if self.source.commit(span, consumed) {
            Ok(())
        } else {
            Err(self.latch(Failure::Backend(BackendFault::ImpossibleState)))
        }
    }

    pub(crate) fn need_input(&self, progress: Progress) -> Result<Step, OperationError> {
        self.require_active()?;
        Ok(Step::new(progress, Status::NeedInput))
    }

    /// Closes input while allowing bounded final output to drain.
    ///
    /// Returns `true` when the state was already complete.
    pub(crate) fn begin_finish(&mut self) -> Result<bool, OperationError> {
        match self.phase {
            Phase::Active => {
                self.phase = Phase::Finishing;
                Ok(false)
            }
            Phase::Finishing => Ok(false),
            Phase::Complete => Ok(true),
            Phase::Failed(failure) => Err(OperationError::Failed(failure)),
        }
    }

    pub(crate) fn output_full(
        &self,
        progress: Progress,
        minimum_output: NonZeroUsize,
    ) -> Result<Step, OperationError> {
        self.require_in_progress()?;
        Ok(Step::new(
            progress,
            Status::OutputFull(OutputFull::new(minimum_output)),
        ))
    }

    pub(crate) fn finish(&mut self, progress: Progress) -> Result<Step, OperationError> {
        match self.phase {
            Phase::Active | Phase::Finishing => {
                self.phase = Phase::Complete;
                Ok(Step::new(progress, Status::Complete))
            }
            Phase::Complete => Ok(Step::new(Progress::ZERO, Status::Complete)),
            Phase::Failed(failure) => Err(OperationError::Failed(failure)),
        }
    }

    pub(crate) fn fail(&mut self, failure: Failure) -> OperationError {
        match self.phase {
            Phase::Active | Phase::Finishing => self.latch(failure),
            Phase::Complete => OperationError::Terminal(TerminalError::InputAfterComplete),
            Phase::Failed(existing) => OperationError::Failed(existing),
        }
    }

    pub(crate) const fn source_position(&self) -> usize {
        self.source.next
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }

    fn require_active(&self) -> Result<(), OperationError> {
        match self.phase {
            Phase::Active => Ok(()),
            Phase::Finishing => Err(OperationError::Terminal(TerminalError::InputAfterFinish)),
            Phase::Complete => Err(OperationError::Terminal(TerminalError::InputAfterComplete)),
            Phase::Failed(failure) => Err(OperationError::Failed(failure)),
        }
    }

    fn require_in_progress(&self) -> Result<(), OperationError> {
        match self.phase {
            Phase::Active | Phase::Finishing => Ok(()),
            Phase::Complete => Err(OperationError::Terminal(TerminalError::InputAfterComplete)),
            Phase::Failed(failure) => Err(OperationError::Failed(failure)),
        }
    }

    fn latch(&mut self, failure: Failure) -> OperationError {
        self.phase = Phase::Failed(failure);
        OperationError::Failed(failure)
    }

    #[cfg(test)]
    pub(crate) const fn at_source_position(next: usize) -> Self {
        Self {
            phase: Phase::Active,
            source: SourcePosition { next },
        }
    }
}
