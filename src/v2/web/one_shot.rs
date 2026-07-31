//! Transactional one-shot WHATWG forgiving decode operations.

use super::{ForgivingBase64, ForgivingError};
use crate::v2::Status;

impl ForgivingBase64 {
    /// Validates one web string without returning decoded bytes.
    pub fn validate(self, input: &str) -> Result<(), ForgivingError> {
        self.decoded_len(input).map(|_| ())
    }

    /// Validates a web string and returns its exact decoded byte length.
    pub fn decoded_len(self, input: &str) -> Result<usize, ForgivingError> {
        measure(input)
    }

    /// Decodes a web string into a caller-owned destination transactionally.
    ///
    /// Full WHATWG validation and sizing happen before the first destination
    /// write. Every returned error leaves the complete destination unchanged.
    pub fn decode_into(self, input: &str, output: &mut [u8]) -> Result<usize, ForgivingError> {
        let required = self.decoded_len(input)?;
        if output.len() < required {
            return Err(ForgivingError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        decode_validated(input, &mut output[..required])?;
        Ok(required)
    }
}

fn measure(input: &str) -> Result<usize, ForgivingError> {
    let mut decoder = ForgivingBase64.decoder();
    let mut offset = 0;
    let mut total = 0usize;
    let mut scratch = [0u8; 3];
    while offset < input.len() {
        let step = decoder.update(&input[offset..], &mut scratch)?;
        let progress = step.progress();
        offset += progress.input_consumed();
        total = total
            .checked_add(progress.output_produced())
            .ok_or(ForgivingError::PositionOverflow)?;
    }
    loop {
        let step = decoder.finish(&mut scratch)?;
        total = total
            .checked_add(step.progress().output_produced())
            .ok_or(ForgivingError::PositionOverflow)?;
        if step.status() == Status::Complete {
            return Ok(total);
        }
    }
}

fn decode_validated(input: &str, output: &mut [u8]) -> Result<(), ForgivingError> {
    let mut decoder = ForgivingBase64.decoder();
    let mut input_offset = 0;
    let mut output_offset = 0;
    while input_offset < input.len() {
        let step = decoder.update(&input[input_offset..], &mut output[output_offset..])?;
        input_offset += step.progress().input_consumed();
        output_offset += step.progress().output_produced();
    }
    loop {
        let step = decoder.finish(&mut output[output_offset..])?;
        output_offset += step.progress().output_produced();
        if step.status() == Status::Complete {
            return Ok(());
        }
    }
}
