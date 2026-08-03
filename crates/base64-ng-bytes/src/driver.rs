use base64_ng::{DecoderState, EncoderState, OperationError, Status};
use bytes::{Buf, BufMut};

use crate::{BytesError, BytesErrorKind, BytesLimits, BytesProgress, BytesStep};

const SCRATCH_LEN: usize = 1024;

/// Resumable, prefix-committing encoder for fragmented buffers.
///
/// Successful calls report input advanced and output passed to the destination.
/// Those prefixes cannot be rolled back. A downstream `Buf`/`BufMut` panic
/// permanently latches the adapter failed; catch-unwind users may inspect
/// [`Self::is_failed`] but must reset before reuse.
#[derive(Debug)]
pub struct BytesEncoder {
    driver: Driver,
}

impl BytesEncoder {
    pub(crate) const fn new(state: EncoderState, limits: BytesLimits) -> Self {
        Self {
            driver: Driver::new(State::Encoder(state), limits),
        }
    }

    /// Accepts fragmented input and commits encoded output that fits.
    ///
    /// [`Status::OutputFull`] is retryable with the same state and remaining
    /// input. Returned errors are absorbing until [`Self::reset`].
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] on limit, buffer-contract, or core failures.
    pub fn update<B, M>(&mut self, input: &mut B, output: &mut M) -> Result<BytesStep, BytesError>
    where
        B: Buf,
        M: BufMut,
    {
        self.driver.update(input, output)
    }

    /// Emits the final encoded tail into the fragmented destination.
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] on output-limit or core failures.
    pub fn finish<M>(&mut self, output: &mut M) -> Result<BytesStep, BytesError>
    where
        M: BufMut,
    {
        self.driver.finish(output)
    }

    /// Resets state, cumulative progress, and the absorbing failure latch.
    pub fn reset(&mut self) {
        self.driver.reset();
    }

    /// Returns cumulative input bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.driver.source_position()
    }

    /// Returns cumulative output bytes committed since reset.
    #[must_use]
    pub const fn output_committed(&self) -> usize {
        self.driver.output_committed
    }

    /// Returns whether an error or downstream panic latched the adapter closed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.driver.failed
    }
}

/// Resumable, prefix-committing strict decoder for fragmented buffers.
///
/// This ordinary decoder may commit plaintext before a malformed later suffix
/// is observed. Secret-bearing frames require a bounded validate-before-release
/// API from the core `secrets` capability, not this adapter.
#[derive(Debug)]
pub struct BytesDecoder {
    driver: Driver,
}

impl BytesDecoder {
    pub(crate) const fn new(state: DecoderState, limits: BytesLimits) -> Self {
        Self {
            driver: Driver::new(State::Decoder(state), limits),
        }
    }

    /// Accepts fragmented encoded input and commits decoded output that fits.
    ///
    /// [`Status::OutputFull`] is retryable with the same state and remaining
    /// input. Returned errors are absorbing until [`Self::reset`].
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] on malformed input, limits, buffer-contract, or
    /// core failures.
    pub fn update<B, M>(&mut self, input: &mut B, output: &mut M) -> Result<BytesStep, BytesError>
    where
        B: Buf,
        M: BufMut,
    {
        self.driver.update(input, output)
    }

    /// Performs strict final-length and padding validation.
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] on malformed final input, output limits, or core
    /// failures.
    pub fn finish<M>(&mut self, output: &mut M) -> Result<BytesStep, BytesError>
    where
        M: BufMut,
    {
        self.driver.finish(output)
    }

    /// Resets state, cumulative progress, and the absorbing failure latch.
    pub fn reset(&mut self) {
        self.driver.reset();
    }

    /// Returns cumulative encoded bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.driver.source_position()
    }

    /// Returns cumulative plaintext bytes committed since reset.
    #[must_use]
    pub const fn output_committed(&self) -> usize {
        self.driver.output_committed
    }

    /// Returns whether an error or downstream panic latched the adapter closed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.driver.failed
    }
}

#[derive(Debug)]
struct Driver {
    state: State,
    limits: BytesLimits,
    output_committed: usize,
    failed: bool,
}

#[derive(Debug)]
enum State {
    Encoder(EncoderState),
    Decoder(DecoderState),
}

impl Driver {
    const fn new(state: State, limits: BytesLimits) -> Self {
        Self {
            state,
            limits,
            output_committed: 0,
            failed: false,
        }
    }

    fn update<B, M>(&mut self, input: &mut B, output: &mut M) -> Result<BytesStep, BytesError>
    where
        B: Buf,
        M: BufMut,
    {
        self.require_open()?;
        self.failed = true;
        self.preflight_input(input.remaining(), BytesProgress::ZERO)?;

        let mut progress = BytesProgress::ZERO;
        loop {
            let reported_remaining = input.remaining();
            self.preflight_input(reported_remaining, progress)?;
            let chunk = input.chunk();
            if (reported_remaining != 0 && chunk.is_empty()) || chunk.len() > reported_remaining {
                return Err(BytesError::new(
                    progress,
                    BytesErrorKind::InvalidInputBuffer {
                        remaining: reported_remaining,
                    },
                ));
            }

            let output_budget = self
                .limits
                .max_output_len()
                .saturating_sub(self.output_committed);
            let writable = output.remaining_mut().min(output_budget).min(SCRATCH_LEN);
            let mut scratch = [0u8; SCRATCH_LEN];
            let step = self
                .state
                .update(chunk, &mut scratch[..writable])
                .map_err(|error| BytesError::new(progress, BytesErrorKind::Operation(error)))?;
            let core = step.progress();
            let consumed = core.input_consumed();
            let produced = core.output_produced();
            if consumed > chunk.len() || produced > writable {
                return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
            }

            output.put_slice(&scratch[..produced]);
            self.output_committed = self
                .output_committed
                .checked_add(produced)
                .ok_or_else(|| BytesError::new(progress, BytesErrorKind::ImpossibleState))?;
            if !progress.add_output(produced) {
                return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
            }

            input.advance(consumed);
            if input.remaining() != reported_remaining - consumed || !progress.add_input(consumed) {
                return Err(BytesError::new(
                    progress,
                    BytesErrorKind::InvalidInputBuffer {
                        remaining: input.remaining(),
                    },
                ));
            }

            match step.status() {
                Status::OutputFull(requirement) => {
                    if self.output_committed == self.limits.max_output_len() {
                        return Err(BytesError::new(
                            progress,
                            BytesErrorKind::OutputLimitExceeded {
                                limit: self.limits.max_output_len(),
                            },
                        ));
                    }
                    if output.remaining_mut() == 0 {
                        self.failed = false;
                        return Ok(BytesStep::new(progress, Status::OutputFull(requirement)));
                    }
                    if consumed == 0 && produced == 0 {
                        return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
                    }
                }
                Status::NeedInput => {
                    if input.has_remaining() {
                        if consumed == 0 && produced == 0 {
                            return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
                        }
                    } else {
                        self.failed = false;
                        return Ok(BytesStep::new(progress, Status::NeedInput));
                    }
                }
                Status::Complete => {
                    self.failed = false;
                    return Ok(BytesStep::new(progress, Status::Complete));
                }
                _ => {
                    return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
                }
            }
        }
    }

    fn finish<M>(&mut self, output: &mut M) -> Result<BytesStep, BytesError>
    where
        M: BufMut,
    {
        self.require_open()?;
        self.failed = true;
        let mut progress = BytesProgress::ZERO;

        loop {
            let output_budget = self
                .limits
                .max_output_len()
                .saturating_sub(self.output_committed);
            let writable = output.remaining_mut().min(output_budget).min(SCRATCH_LEN);
            let mut scratch = [0u8; SCRATCH_LEN];
            let step = self
                .state
                .finish(&mut scratch[..writable])
                .map_err(|error| BytesError::new(progress, BytesErrorKind::Operation(error)))?;
            let produced = step.progress().output_produced();
            if step.progress().input_consumed() != 0 || produced > writable {
                return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
            }

            output.put_slice(&scratch[..produced]);
            self.output_committed = self
                .output_committed
                .checked_add(produced)
                .ok_or_else(|| BytesError::new(progress, BytesErrorKind::ImpossibleState))?;
            if !progress.add_output(produced) {
                return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
            }

            match step.status() {
                Status::Complete => {
                    self.failed = false;
                    return Ok(BytesStep::new(progress, Status::Complete));
                }
                Status::OutputFull(requirement) => {
                    if self.output_committed == self.limits.max_output_len() {
                        return Err(BytesError::new(
                            progress,
                            BytesErrorKind::OutputLimitExceeded {
                                limit: self.limits.max_output_len(),
                            },
                        ));
                    }
                    if output.remaining_mut() == 0 {
                        self.failed = false;
                        return Ok(BytesStep::new(progress, Status::OutputFull(requirement)));
                    }
                    if produced == 0 {
                        return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
                    }
                }
                _ => {
                    return Err(BytesError::new(progress, BytesErrorKind::ImpossibleState));
                }
            }
        }
    }

    fn require_open(&self) -> Result<(), BytesError> {
        if self.failed {
            Err(BytesError::new(
                BytesProgress::ZERO,
                BytesErrorKind::FailedState,
            ))
        } else {
            Ok(())
        }
    }

    fn preflight_input(
        &mut self,
        incoming: usize,
        progress: BytesProgress,
    ) -> Result<(), BytesError> {
        let Some(required) = self.source_position().checked_add(incoming) else {
            self.failed = true;
            return Err(BytesError::new(progress, BytesErrorKind::LengthOverflow));
        };
        if required > self.limits.max_input_len() {
            self.failed = true;
            return Err(BytesError::new(
                progress,
                BytesErrorKind::InputLimitExceeded {
                    required,
                    limit: self.limits.max_input_len(),
                },
            ));
        }
        Ok(())
    }

    const fn source_position(&self) -> usize {
        match &self.state {
            State::Encoder(state) => state.source_position(),
            State::Decoder(state) => state.source_position(),
        }
    }

    fn reset(&mut self) {
        match &mut self.state {
            State::Encoder(state) => state.reset(),
            State::Decoder(state) => state.reset(),
        }
        self.output_committed = 0;
        self.failed = false;
    }
}

impl State {
    fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<base64_ng::Step, OperationError> {
        match self {
            Self::Encoder(state) => state.update(input, output),
            Self::Decoder(state) => state.update(input, output),
        }
    }

    fn finish(&mut self, output: &mut [u8]) -> Result<base64_ng::Step, OperationError> {
        match self {
            Self::Encoder(state) => state.finish(output),
            Self::Decoder(state) => state.finish(output),
        }
    }
}
