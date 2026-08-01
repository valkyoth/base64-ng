//! Explicit legacy ASCII-whitespace compatibility decoding.

use super::{
    contracts::{BackendFault, OperationError, Status, Step},
    incremental_decoder::DecoderState,
    ordinary::{OneShotError, map_operation_error},
    specifications::{Base64, Codec},
};

/// The one retained legacy transport-whitespace decode policy.
///
/// It ignores only ASCII space, tab, carriage return, and line feed. It is an
/// ordinary compatibility policy and is not available through `secret::*`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LegacyAsciiWhitespace;

/// Explicit legacy decoder policy value.
pub const ASCII_WHITESPACE: LegacyAsciiWhitespace = LegacyAsciiWhitespace;

/// Heapless ordinary decoder that ignores the documented legacy whitespace.
///
/// Detailed malformed-input errors retain indexes in the original source,
/// including across whitespace-only chunks. The state does not wipe on drop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyWhitespaceDecoder {
    inner: DecoderState,
}

impl LegacyAsciiWhitespace {
    /// Constructs a decoder for one explicit codec.
    pub fn decoder<S: Codec>(&self, codec: &Base64<S>) -> LegacyWhitespaceDecoder {
        LegacyWhitespaceDecoder {
            inner: DecoderState::new_legacy_ascii_whitespace(codec.settings()),
        }
    }

    /// Validates legacy-whitespace input without producing decoded bytes.
    pub fn validate<S: Codec>(&self, codec: &Base64<S>, input: &[u8]) -> Result<(), OneShotError> {
        self.decoded_len(codec, input).map(|_| ())
    }

    /// Validates input and returns its exact decoded length.
    pub fn decoded_len<S: Codec>(
        &self,
        codec: &Base64<S>,
        input: &[u8],
    ) -> Result<usize, OneShotError> {
        measure(self.decoder(codec), input)
    }

    /// Decodes transactionally after complete validation and exact sizing.
    ///
    /// Every returned error leaves the complete destination unchanged.
    pub fn decode_into<S: Codec>(
        &self,
        codec: &Base64<S>,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, OneShotError> {
        let required = self.decoded_len(codec, input)?;
        if output.len() < required {
            return Err(OneShotError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        decode_validated(self.decoder(codec), input, &mut output[..required])?;
        Ok(required)
    }
}

impl LegacyWhitespaceDecoder {
    /// Accepts one original source fragment and writes decoded bytes that fit.
    pub fn update(&mut self, input: &[u8], output: &mut [u8]) -> Result<Step, OperationError> {
        self.inner.update(input, output)
    }

    /// Finalizes the selected codec's padding policy.
    pub fn finish(&mut self, output: &mut [u8]) -> Result<Step, OperationError> {
        self.inner.finish(output)
    }

    /// Resets the state for an unrelated ordinary message.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Returns the number of original source bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.inner.source_position()
    }

    #[cfg(test)]
    pub(crate) fn set_source_position_for_test(&mut self, source_position: usize) {
        self.inner.set_source_position_for_test(source_position);
    }
}

fn measure(mut decoder: LegacyWhitespaceDecoder, input: &[u8]) -> Result<usize, OneShotError> {
    let mut input_offset = 0;
    let mut output_len = 0usize;
    let mut scratch = [0u8; 3];
    while input_offset < input.len() {
        let step = decoder
            .update(&input[input_offset..], &mut scratch)
            .map_err(map_operation_error)?;
        let progress = step.progress();
        if progress.input_consumed() == 0 && progress.output_produced() == 0 {
            return Err(OneShotError::Backend(BackendFault::ImpossibleState));
        }
        input_offset += progress.input_consumed();
        output_len = output_len
            .checked_add(progress.output_produced())
            .ok_or(OneShotError::LengthOverflow)?;
    }
    finish_measurement(&mut decoder, output_len, &mut scratch)
}

fn finish_measurement(
    decoder: &mut LegacyWhitespaceDecoder,
    mut output_len: usize,
    scratch: &mut [u8; 3],
) -> Result<usize, OneShotError> {
    loop {
        let step = decoder.finish(scratch).map_err(map_operation_error)?;
        output_len = output_len
            .checked_add(step.progress().output_produced())
            .ok_or(OneShotError::LengthOverflow)?;
        match step.status() {
            Status::Complete => return Ok(output_len),
            Status::OutputFull(_) if step.progress().output_produced() != 0 => {}
            Status::OutputFull(_) | Status::NeedInput => {
                return Err(OneShotError::Backend(BackendFault::ImpossibleState));
            }
        }
    }
}

fn decode_validated(
    mut decoder: LegacyWhitespaceDecoder,
    input: &[u8],
    output: &mut [u8],
) -> Result<(), OneShotError> {
    let mut input_offset = 0;
    let mut output_offset = 0;
    while input_offset < input.len() {
        let step = decoder
            .update(&input[input_offset..], &mut output[output_offset..])
            .map_err(map_operation_error)?;
        let progress = step.progress();
        if progress.input_consumed() == 0 && progress.output_produced() == 0 {
            return Err(OneShotError::Backend(BackendFault::ImpossibleState));
        }
        input_offset += progress.input_consumed();
        output_offset += progress.output_produced();
    }
    loop {
        let step = decoder
            .finish(&mut output[output_offset..])
            .map_err(map_operation_error)?;
        output_offset += step.progress().output_produced();
        match step.status() {
            Status::Complete => return Ok(()),
            Status::OutputFull(_) if step.progress().output_produced() != 0 => {}
            Status::OutputFull(_) | Status::NeedInput => {
                return Err(OneShotError::Backend(BackendFault::ImpossibleState));
            }
        }
    }
}
