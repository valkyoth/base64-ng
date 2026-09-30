//! Reference preflight and the private canonical writer.

use super::{
    contracts::{BackendFault, Status},
    incremental_decoder::DecoderState,
    ordinary::{OneShotError, map_operation_error},
    specifications::{CodecSettings, DecodePadding},
};
use crate::{
    DecodeValidation,
    decode_preflight::{Failure as PreflightFailure, Preflight},
};

pub(super) fn prepare(
    settings: CodecSettings,
    input: &[u8],
    validation: DecodeValidation,
) -> Result<Preflight<'_, CodecSettings>, OneShotError> {
    match validation {
        DecodeValidation::Auto | DecodeValidation::ScalarReference => {
            Preflight::reference(input, settings, validate_and_measure).map_err(map_preflight_error)
        }
    }
}

fn map_preflight_error(error: PreflightFailure<OneShotError>) -> OneShotError {
    match error {
        PreflightFailure::Input(error) => error,
        PreflightFailure::Bounds | PreflightFailure::ClassifierDisagreement => {
            OneShotError::Backend(BackendFault::ImpossibleState)
        }
    }
}

fn validate_and_measure(settings: CodecSettings, input: &[u8]) -> Result<usize, OneShotError> {
    #[cfg(test)]
    crate::decode_validation::observation::record();
    let mut decoder = if settings.decode_padding() == DecodePadding::RequireCanonical {
        DecoderState::new_padded(settings)
    } else {
        DecoderState::new_unpadded(settings)
    };
    let mut input_offset = 0;
    let mut measured_len = 0usize;
    let mut scratch = [0u8; 3];
    while input_offset < input.len() {
        let step = decoder
            .update(&input[input_offset..], &mut scratch)
            .map_err(map_operation_error)?;
        let progress = step.progress();
        if progress.input_consumed() == 0 && progress.output_produced() == 0 {
            return Err(OneShotError::Backend(BackendFault::ImpossibleState));
        }
        if progress.input_consumed() > input.len() - input_offset
            || progress.output_produced() > scratch.len()
        {
            return Err(OneShotError::Backend(BackendFault::ImpossibleState));
        }
        input_offset += progress.input_consumed();
        measured_len = measured_len
            .checked_add(progress.output_produced())
            .ok_or(OneShotError::LengthOverflow)?;
    }
    loop {
        let step = decoder.finish(&mut scratch).map_err(map_operation_error)?;
        if step.progress().input_consumed() != 0
            || step.progress().output_produced() > scratch.len()
        {
            return Err(OneShotError::Backend(BackendFault::ImpossibleState));
        }
        measured_len = measured_len
            .checked_add(step.progress().output_produced())
            .ok_or(OneShotError::LengthOverflow)?;
        match step.status() {
            Status::Complete => return Ok(measured_len),
            Status::OutputFull(_) if step.progress().output_produced() != 0 => {}
            _ => return Err(OneShotError::Backend(BackendFault::ImpossibleState)),
        }
    }
}

pub(super) fn write(
    proof: Preflight<'_, CodecSettings>,
    output: &mut [u8],
) -> Result<usize, OneShotError> {
    proof
        .write(
            output,
            |settings, interior, tail, body_output, tail_output| {
                for (input, output) in interior
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(body_output.as_chunks_mut::<3>().0.iter_mut())
                {
                    let first = value(settings, input[0]);
                    let second = value(settings, input[1]);
                    let third = value(settings, input[2]);
                    output[0] = (first << 2) | (second >> 4);
                    output[1] = (second << 4) | (third >> 2);
                    output[2] = (third << 6) | value(settings, input[3]);
                }
                if !tail_output.is_empty() {
                    let first = value(settings, tail[0]);
                    let second = value(settings, tail[1]);
                    tail_output[0] = (first << 2) | (second >> 4);
                    if tail_output.len() > 1 {
                        let third = value(settings, tail[2]);
                        tail_output[1] = (second << 4) | (third >> 2);
                        if tail_output.len() > 2 {
                            tail_output[2] = (third << 6) | value(settings, tail[3]);
                        }
                    }
                }
            },
        )
        .map_err(|error| OneShotError::OutputTooSmall {
            required: error.required,
            available: error.available,
        })
}

fn value(settings: CodecSettings, byte: u8) -> u8 {
    settings.alphabet().decode_byte(byte).unwrap_or(0)
}

#[cfg(test)]
mod tests;
