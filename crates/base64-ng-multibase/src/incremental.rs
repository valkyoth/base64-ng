use core::num::NonZeroUsize;

use base64_ng::{DecoderState, EncoderState, Status};

use crate::{
    Base64MultibaseEncoding, Base64MultibaseError, Base64MultibaseErrorKind, Base64MultibaseLimits,
    Base64MultibaseStatus, Base64MultibaseStep, error::map_operation,
};

/// Heapless bounded incremental Base64-family multibase encoder.
///
/// The state retains only the shared core encoder quantum and one pending
/// prefix bit of state. It is an ordinary, non-wiping transform.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Base64MultibaseEncoder {
    encoding: Base64MultibaseEncoding,
    limits: Base64MultibaseLimits,
    core: EncoderState,
    prefix_pending: bool,
    input_accepted: usize,
    output_produced: usize,
    limit_blocked: bool,
    terminal: bool,
}

impl Base64MultibaseEncoder {
    /// Creates an encoder for one exact registered Base64-family prefix.
    ///
    /// # Errors
    ///
    /// Returns [`Base64MultibaseErrorKind::OutputLimitExceeded`] if the output
    /// limit cannot hold even the required prefix.
    pub fn new(
        encoding: Base64MultibaseEncoding,
        limits: Base64MultibaseLimits,
    ) -> Result<Self, Base64MultibaseError> {
        if limits.max_output_bytes() == 0 {
            return Err(Base64MultibaseError::new(
                Base64MultibaseErrorKind::OutputLimitExceeded,
            ));
        }
        Ok(Self {
            encoding,
            limits,
            core: encoding.codec().encoder(),
            prefix_pending: true,
            input_accepted: 0,
            output_produced: 0,
            limit_blocked: false,
            terminal: false,
        })
    }

    /// Accepts a payload prefix and writes as much prefixed output as fits.
    ///
    /// # Errors
    ///
    /// Returns [`Base64MultibaseError`] for terminal state, finite-limit,
    /// position, or shared backend failure. Any error latches the state.
    pub fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<Base64MultibaseStep, Base64MultibaseError> {
        self.require_active()?;
        self.preflight_input(input.len())?;
        if self.limit_blocked {
            return self.fail(Base64MultibaseErrorKind::OutputLimitExceeded);
        }

        let mut produced = self.emit_prefix(output);
        if self.prefix_pending {
            return Ok(Base64MultibaseStep::new(
                0,
                produced,
                Base64MultibaseStatus::OutputFull(NonZeroUsize::MIN),
            ));
        }
        let available = self.available_output(output.len().saturating_sub(produced));
        let step = match self
            .core
            .update(input, &mut output[produced..produced + available])
        {
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
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        produced = produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, Base64MultibaseStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        Ok(Base64MultibaseStep::new(
            progress.input_consumed(),
            produced,
            status,
        ))
    }

    /// Emits the canonical tail and completes the value.
    ///
    /// # Errors
    ///
    /// Returns [`Base64MultibaseError`] for terminal state, output limit, or
    /// shared backend failure. Any error latches the state.
    pub fn finish(
        &mut self,
        output: &mut [u8],
    ) -> Result<Base64MultibaseStep, Base64MultibaseError> {
        self.require_active()?;
        if self.limit_blocked {
            return self.fail(Base64MultibaseErrorKind::OutputLimitExceeded);
        }
        let mut produced = self.emit_prefix(output);
        if self.prefix_pending {
            return Ok(Base64MultibaseStep::new(
                0,
                produced,
                Base64MultibaseStatus::OutputFull(NonZeroUsize::MIN),
            ));
        }
        let available = self.available_output(output.len().saturating_sub(produced));
        let step = match self
            .core
            .finish(&mut output[produced..produced + available])
        {
            Ok(step) => step,
            Err(error) => {
                self.terminal = true;
                return Err(map_operation(error));
            }
        };
        let progress = step.progress();
        produced = produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, Base64MultibaseStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        self.terminal = status == Base64MultibaseStatus::Complete;
        Ok(Base64MultibaseStep::new(0, produced, status))
    }

    /// Resets the state for an unrelated payload with the same encoding.
    pub fn reset(&mut self) {
        self.core.reset();
        self.prefix_pending = true;
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Clears retained ordinary core bytes and resets the state.
    pub fn clear(&mut self) {
        self.core.clear();
        self.prefix_pending = true;
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Returns the selected encoding.
    #[must_use]
    pub const fn encoding(&self) -> Base64MultibaseEncoding {
        self.encoding
    }

    /// Returns payload bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.input_accepted
    }

    fn emit_prefix(&mut self, output: &mut [u8]) -> usize {
        if !self.prefix_pending || output.is_empty() {
            return 0;
        }
        output[0] = self.encoding.prefix();
        self.prefix_pending = false;
        self.output_produced += 1;
        1
    }

    fn available_output(&self, caller_available: usize) -> usize {
        caller_available.min(
            self.limits
                .max_output_bytes()
                .saturating_sub(self.output_produced),
        )
    }

    fn preflight_input(&mut self, additional: usize) -> Result<(), Base64MultibaseError> {
        let Some(required) = self.input_accepted.checked_add(additional) else {
            return self.fail(Base64MultibaseErrorKind::LengthOverflow);
        };
        if required > self.limits.max_input_bytes() {
            return self.fail(Base64MultibaseErrorKind::InputLimitExceeded);
        }
        if required > self.limits.max_work_before_output() {
            return self.fail(Base64MultibaseErrorKind::WorkLimitExceeded);
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), Base64MultibaseError> {
        if self.terminal {
            Err(Base64MultibaseError::new(
                Base64MultibaseErrorKind::TerminalState,
            ))
        } else {
            Ok(())
        }
    }

    fn fail<T>(&mut self, kind: Base64MultibaseErrorKind) -> Result<T, Base64MultibaseError> {
        Err(self.latch(kind))
    }

    fn latch(&mut self, kind: Base64MultibaseErrorKind) -> Base64MultibaseError {
        self.terminal = true;
        Base64MultibaseError::new(kind)
    }
}

/// Heapless bounded incremental Base64-family multibase decoder.
///
/// The first input byte selects one of four strict canonical core decoders.
/// Previously released ordinary plaintext cannot be retracted after a later
/// malformed chunk; use one-shot decoding when transactional release matters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Base64MultibaseDecoder {
    limits: Base64MultibaseLimits,
    state: DecoderMode,
    input_accepted: usize,
    output_produced: usize,
    limit_blocked: bool,
    terminal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DecoderMode {
    AwaitingPrefix,
    Active {
        encoding: Base64MultibaseEncoding,
        core: DecoderState,
    },
}

impl Base64MultibaseDecoder {
    /// Creates a decoder awaiting its exact case-sensitive prefix.
    #[must_use]
    pub const fn new(limits: Base64MultibaseLimits) -> Self {
        Self {
            limits,
            state: DecoderMode::AwaitingPrefix,
            input_accepted: 0,
            output_produced: 0,
            limit_blocked: false,
            terminal: false,
        }
    }

    /// Accepts prefixed input and writes as much decoded output as fits.
    ///
    /// # Errors
    ///
    /// Returns [`Base64MultibaseError`] for prefix, payload, finite-limit,
    /// terminal-state, position, or backend failure. Any error latches state.
    pub fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<Base64MultibaseStep, Base64MultibaseError> {
        self.require_active()?;
        self.preflight_input(input.len())?;
        if self.limit_blocked {
            return self.fail(Base64MultibaseErrorKind::OutputLimitExceeded);
        }

        let (prefix_consumed, payload) = if matches!(self.state, DecoderMode::AwaitingPrefix) {
            let Some((&prefix, payload)) = input.split_first() else {
                return Ok(Base64MultibaseStep::new(
                    0,
                    0,
                    Base64MultibaseStatus::NeedInput,
                ));
            };
            let Some(encoding) = Base64MultibaseEncoding::from_prefix(prefix) else {
                self.terminal = true;
                return Err(Base64MultibaseError::unsupported(prefix));
            };
            self.state = DecoderMode::Active {
                encoding,
                core: encoding.codec().decoder(),
            };
            (1, payload)
        } else {
            (0, input)
        };

        if payload.is_empty() {
            self.input_accepted += prefix_consumed;
            return Ok(Base64MultibaseStep::new(
                prefix_consumed,
                0,
                Base64MultibaseStatus::NeedInput,
            ));
        }
        let available = output.len().min(
            self.limits
                .max_output_bytes()
                .saturating_sub(self.output_produced),
        );
        let step = match &mut self.state {
            DecoderMode::AwaitingPrefix => {
                return self.fail(Base64MultibaseErrorKind::BackendFailure);
            }
            DecoderMode::Active { core, .. } => core
                .update(payload, &mut output[..available])
                .map_err(map_operation),
        };
        let step = match step {
            Ok(step) => step,
            Err(error) => {
                self.terminal = true;
                return Err(error);
            }
        };
        let progress = step.progress();
        let consumed = prefix_consumed
            .checked_add(progress.input_consumed())
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        self.input_accepted = self
            .input_accepted
            .checked_add(consumed)
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, Base64MultibaseStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        Ok(Base64MultibaseStep::new(
            consumed,
            progress.output_produced(),
            status,
        ))
    }

    /// Finalizes the prefix-selected payload.
    ///
    /// # Errors
    ///
    /// Returns [`Base64MultibaseError`] for a missing prefix, malformed tail,
    /// output limit, terminal state, or backend failure.
    pub fn finish(
        &mut self,
        output: &mut [u8],
    ) -> Result<Base64MultibaseStep, Base64MultibaseError> {
        self.require_active()?;
        if self.limit_blocked {
            return self.fail(Base64MultibaseErrorKind::OutputLimitExceeded);
        }
        let available = output.len().min(
            self.limits
                .max_output_bytes()
                .saturating_sub(self.output_produced),
        );
        let step = match &mut self.state {
            DecoderMode::AwaitingPrefix => {
                return self.fail(Base64MultibaseErrorKind::MissingPrefix);
            }
            DecoderMode::Active { core, .. } => {
                core.finish(&mut output[..available]).map_err(map_operation)
            }
        };
        let step = match step {
            Ok(step) => step,
            Err(error) => {
                self.terminal = true;
                return Err(error);
            }
        };
        let progress = step.progress();
        self.output_produced = self
            .output_produced
            .checked_add(progress.output_produced())
            .ok_or_else(|| self.latch(Base64MultibaseErrorKind::LengthOverflow))?;
        let status = map_status(step.status()).map_err(|kind| self.latch(kind))?;
        self.limit_blocked = matches!(status, Base64MultibaseStatus::OutputFull(_))
            && self.output_produced == self.limits.max_output_bytes();
        self.terminal = status == Base64MultibaseStatus::Complete;
        Ok(Base64MultibaseStep::new(
            0,
            progress.output_produced(),
            status,
        ))
    }

    /// Resets the state to await an unrelated prefixed value.
    pub fn reset(&mut self) {
        if let DecoderMode::Active { core, .. } = &mut self.state {
            core.reset();
        }
        self.state = DecoderMode::AwaitingPrefix;
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Clears retained ordinary core bytes and resets the state.
    pub fn clear(&mut self) {
        if let DecoderMode::Active { core, .. } = &mut self.state {
            core.clear();
        }
        self.state = DecoderMode::AwaitingPrefix;
        self.input_accepted = 0;
        self.output_produced = 0;
        self.limit_blocked = false;
        self.terminal = false;
    }

    /// Returns the selected encoding after a prefix has been accepted.
    #[must_use]
    pub const fn encoding(&self) -> Option<Base64MultibaseEncoding> {
        match self.state {
            DecoderMode::AwaitingPrefix => None,
            DecoderMode::Active { encoding, .. } => Some(encoding),
        }
    }

    /// Returns complete multibase input bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.input_accepted
    }

    fn preflight_input(&mut self, additional: usize) -> Result<(), Base64MultibaseError> {
        let Some(required) = self.input_accepted.checked_add(additional) else {
            return self.fail(Base64MultibaseErrorKind::LengthOverflow);
        };
        if required > self.limits.max_input_bytes() {
            return self.fail(Base64MultibaseErrorKind::InputLimitExceeded);
        }
        if required > self.limits.max_work_before_output() {
            return self.fail(Base64MultibaseErrorKind::WorkLimitExceeded);
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), Base64MultibaseError> {
        if self.terminal {
            Err(Base64MultibaseError::new(
                Base64MultibaseErrorKind::TerminalState,
            ))
        } else {
            Ok(())
        }
    }

    fn fail<T>(&mut self, kind: Base64MultibaseErrorKind) -> Result<T, Base64MultibaseError> {
        Err(self.latch(kind))
    }

    fn latch(&mut self, kind: Base64MultibaseErrorKind) -> Base64MultibaseError {
        self.terminal = true;
        Base64MultibaseError::new(kind)
    }
}

fn map_status(status: Status) -> Result<Base64MultibaseStatus, Base64MultibaseErrorKind> {
    match status {
        Status::NeedInput => Ok(Base64MultibaseStatus::NeedInput),
        Status::OutputFull(full) => Ok(Base64MultibaseStatus::OutputFull(full.minimum_output())),
        Status::Complete => Ok(Base64MultibaseStatus::Complete),
        _ => Err(Base64MultibaseErrorKind::BackendFailure),
    }
}
