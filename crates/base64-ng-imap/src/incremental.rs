use base64_ng::{DecoderState, EncoderState, Status};

use crate::{
    ImapPayloadError, ImapPayloadErrorKind, ImapPayloadLimits, ImapPayloadStatus, ImapPayloadStep,
    error::map_operation,
};

/// Heapless bounded incremental modified-Base64 payload encoder.
///
/// Source bytes are already-converted UTF-16BE storage. Previously emitted
/// ordinary payload bytes cannot be retracted if finalization later discovers
/// an odd source length; use one-shot encoding for transactional output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifiedUtf7PayloadEncoder {
    core: EncoderState,
    limits: ImapPayloadLimits,
    input_accepted: usize,
    output_produced: usize,
    limit_blocked: bool,
    terminal: bool,
}

impl ModifiedUtf7PayloadEncoder {
    /// Creates an empty encoder.
    #[must_use]
    pub fn new(limits: ImapPayloadLimits) -> Self {
        Self {
            core: base64_ng::IMAP_MUTF7_ALPHABET_NO_PAD.encoder(),
            limits,
            input_accepted: 0,
            output_produced: 0,
            limit_blocked: false,
            terminal: false,
        }
    }

    /// Accepts UTF-16BE bytes and emits as much payload as fits.
    ///
    /// # Errors
    ///
    /// Returns [`ImapPayloadError`] for terminal state, finite limits,
    /// position overflow, or shared backend failure. Any error latches state.
    pub fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<ImapPayloadStep, ImapPayloadError> {
        self.require_active()?;
        self.preflight_input(input.len())?;
        if self.limit_blocked {
            return self.fail(ImapPayloadErrorKind::OutputLimitExceeded);
        }
        let available = self.available_output(output.len());
        let step = match self.core.update(input, &mut output[..available]) {
            Ok(step) => step,
            Err(error) => {
                self.terminal = true;
                return Err(map_operation(error));
            }
        };
        let progress = step.progress();
        self.input_accepted = self
            .input_accepted
            .checked_add(progress.input_consumed())
            .ok_or_else(|| self.latch(ImapPayloadErrorKind::LengthOverflow))?;
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(ImapPayloadErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, ImapPayloadStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        Ok(ImapPayloadStep::new(
            progress.input_consumed(),
            progress.output_produced(),
            status,
        ))
    }

    /// Emits the canonical unpadded tail and completes the payload.
    ///
    /// # Errors
    ///
    /// Returns [`ImapPayloadError`] for odd UTF-16BE storage, terminal state,
    /// output limits, or shared backend failure. Any error latches state.
    pub fn finish(&mut self, output: &mut [u8]) -> Result<ImapPayloadStep, ImapPayloadError> {
        self.require_active()?;
        if !self.input_accepted.is_multiple_of(2) {
            return self.fail(ImapPayloadErrorKind::InvalidUtf16BeLength);
        }
        if self.limit_blocked {
            return self.fail(ImapPayloadErrorKind::OutputLimitExceeded);
        }
        let available = self.available_output(output.len());
        let step = match self.core.finish(&mut output[..available]) {
            Ok(step) => step,
            Err(error) => {
                self.terminal = true;
                return Err(map_operation(error));
            }
        };
        let progress = step.progress();
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(ImapPayloadErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, ImapPayloadStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        self.terminal = status == ImapPayloadStatus::Complete;
        Ok(ImapPayloadStep::new(0, progress.output_produced(), status))
    }

    /// Resets the state for an unrelated payload with the same limits.
    pub fn reset(&mut self) {
        self.core.reset();
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Clears retained ordinary core bytes and resets the state.
    pub fn clear(&mut self) {
        self.core.clear();
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Returns UTF-16BE source bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.input_accepted
    }

    fn available_output(&self, caller_available: usize) -> usize {
        caller_available.min(
            self.limits
                .max_output_bytes()
                .saturating_sub(self.output_produced),
        )
    }

    fn preflight_input(&mut self, additional: usize) -> Result<(), ImapPayloadError> {
        let Some(required) = self.input_accepted.checked_add(additional) else {
            return self.fail(ImapPayloadErrorKind::LengthOverflow);
        };
        if required > self.limits.max_input_bytes() {
            return self.fail(ImapPayloadErrorKind::InputLimitExceeded);
        }
        if required > self.limits.max_work_before_output() {
            return self.fail(ImapPayloadErrorKind::WorkLimitExceeded);
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), ImapPayloadError> {
        if self.terminal {
            Err(ImapPayloadError::new(ImapPayloadErrorKind::TerminalState))
        } else {
            Ok(())
        }
    }

    fn fail<T>(&mut self, kind: ImapPayloadErrorKind) -> Result<T, ImapPayloadError> {
        Err(self.latch(kind))
    }

    fn latch(&mut self, kind: ImapPayloadErrorKind) -> ImapPayloadError {
        self.terminal = true;
        ImapPayloadError::new(kind)
    }
}

/// Heapless bounded incremental modified-Base64 payload decoder.
///
/// Previously released ordinary UTF-16BE bytes cannot be retracted after a
/// later malformed tail or odd decoded length; use one-shot decoding when the
/// destination must remain unchanged on every error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifiedUtf7PayloadDecoder {
    core: DecoderState,
    limits: ImapPayloadLimits,
    input_accepted: usize,
    output_produced: usize,
    limit_blocked: bool,
    terminal: bool,
}

impl ModifiedUtf7PayloadDecoder {
    /// Creates an empty decoder.
    #[must_use]
    pub fn new(limits: ImapPayloadLimits) -> Self {
        Self {
            core: base64_ng::IMAP_MUTF7_ALPHABET_NO_PAD.decoder(),
            limits,
            input_accepted: 0,
            output_produced: 0,
            limit_blocked: false,
            terminal: false,
        }
    }

    /// Accepts payload text and emits as many UTF-16BE bytes as fit.
    ///
    /// # Errors
    ///
    /// Returns [`ImapPayloadError`] for malformed input, terminal state,
    /// finite limits, or shared backend failure. Any error latches state.
    pub fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<ImapPayloadStep, ImapPayloadError> {
        self.require_active()?;
        self.preflight_input(input.len())?;
        if self.limit_blocked {
            return self.fail(ImapPayloadErrorKind::OutputLimitExceeded);
        }
        let available = self.available_output(output.len());
        let step = match self.core.update(input, &mut output[..available]) {
            Ok(step) => step,
            Err(error) => {
                self.terminal = true;
                return Err(map_operation(error));
            }
        };
        let progress = step.progress();
        self.input_accepted = self
            .input_accepted
            .checked_add(progress.input_consumed())
            .ok_or_else(|| self.latch(ImapPayloadErrorKind::LengthOverflow))?;
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(ImapPayloadErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, ImapPayloadStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        Ok(ImapPayloadStep::new(
            progress.input_consumed(),
            progress.output_produced(),
            status,
        ))
    }

    /// Validates the tail and completes the payload.
    ///
    /// # Errors
    ///
    /// Returns [`ImapPayloadError`] for malformed tails, odd decoded length,
    /// terminal state, output limits, or backend failure.
    pub fn finish(&mut self, output: &mut [u8]) -> Result<ImapPayloadStep, ImapPayloadError> {
        self.require_active()?;
        if self.limit_blocked {
            return self.fail(ImapPayloadErrorKind::OutputLimitExceeded);
        }
        let available = self.available_output(output.len());
        let step = match self.core.finish(&mut output[..available]) {
            Ok(step) => step,
            Err(error) => {
                self.terminal = true;
                return Err(map_operation(error));
            }
        };
        let progress = step.progress();
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(ImapPayloadErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, ImapPayloadStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        if status == ImapPayloadStatus::Complete && !self.output_produced.is_multiple_of(2) {
            return self.fail(ImapPayloadErrorKind::InvalidUtf16BeLength);
        }
        self.terminal = status == ImapPayloadStatus::Complete;
        Ok(ImapPayloadStep::new(0, progress.output_produced(), status))
    }

    /// Resets the state for an unrelated payload with the same limits.
    pub fn reset(&mut self) {
        self.core.reset();
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Clears retained ordinary core bytes and resets the state.
    pub fn clear(&mut self) {
        self.core.clear();
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Returns payload source bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.input_accepted
    }

    fn available_output(&self, caller_available: usize) -> usize {
        caller_available.min(
            self.limits
                .max_output_bytes()
                .saturating_sub(self.output_produced),
        )
    }

    fn preflight_input(&mut self, additional: usize) -> Result<(), ImapPayloadError> {
        let Some(required) = self.input_accepted.checked_add(additional) else {
            return self.fail(ImapPayloadErrorKind::LengthOverflow);
        };
        if required > self.limits.max_input_bytes() {
            return self.fail(ImapPayloadErrorKind::InputLimitExceeded);
        }
        if required > self.limits.max_work_before_output() {
            return self.fail(ImapPayloadErrorKind::WorkLimitExceeded);
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), ImapPayloadError> {
        if self.terminal {
            Err(ImapPayloadError::new(ImapPayloadErrorKind::TerminalState))
        } else {
            Ok(())
        }
    }

    fn fail<T>(&mut self, kind: ImapPayloadErrorKind) -> Result<T, ImapPayloadError> {
        Err(self.latch(kind))
    }

    fn latch(&mut self, kind: ImapPayloadErrorKind) -> ImapPayloadError {
        self.terminal = true;
        ImapPayloadError::new(kind)
    }
}

fn map_status(status: Status) -> Result<ImapPayloadStatus, ImapPayloadErrorKind> {
    match status {
        Status::NeedInput => Ok(ImapPayloadStatus::NeedInput),
        Status::OutputFull(full) => Ok(ImapPayloadStatus::OutputFull(full.minimum_output())),
        Status::Complete => Ok(ImapPayloadStatus::Complete),
        _ => Err(ImapPayloadErrorKind::BackendFailure),
    }
}
