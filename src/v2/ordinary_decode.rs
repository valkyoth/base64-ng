//! Ordinary preflight and the private canonical writer.

use super::{
    contracts::{BackendFault, Status},
    incremental_decoder::DecoderState,
    ordinary::{OneShotError, map_operation_error},
    ordinary_scalar::Family,
    specifications::{CodecSettings, DecodePadding},
};
use crate::{
    DecodeValidation,
    decode_preflight::{Failure as PreflightFailure, Preflight},
};

mod vector;

#[derive(Clone, Copy)]
pub(crate) struct Prepared {
    settings: CodecSettings,
    backend: Option<crate::runtime::Backend>,
}

#[inline]
pub(crate) fn accelerated(input_len: usize) -> bool {
    vector::select(input_len).is_some()
}

pub(crate) fn prepare(
    settings: CodecSettings,
    input: &[u8],
    validation: DecodeValidation,
) -> Result<Preflight<'_, Prepared>, OneShotError> {
    let family = if input.is_empty() {
        None
    } else {
        Family::for_settings(settings)
    };
    let backend = family.and_then(|_| vector::select(input.len()));
    let config = Prepared { settings, backend };
    Preflight::validate(input, config, |config, input| {
        let settings = config.settings;
        // Empty input is valid for every sealed codec, without inspecting its
        // alphabet. Keep the explicit reference policy on the original path.
        if validation == DecodeValidation::Auto && input.is_empty() {
            return Ok(0);
        }
        if validation == DecodeValidation::Auto
            && let Some(family) = family
        {
            let padded = settings.decode_padding() == DecodePadding::RequireCanonical;
            let required = match config.backend {
                Some(backend) => vector::validated_len(backend, family, input, padded),
                None => family.validated_len(input, padded),
            };
            if let Some(required) = required {
                // Checked builds independently validate the entire immutable
                // source before any caller-visible output, not just chunks.
                #[cfg(feature = "checked-backend")]
                if let Some(backend) = config.backend {
                    match validate_and_measure(settings, input) {
                        Ok(reference) if reference == required => {}
                        _ => {
                            vector::quarantine(backend, BackendFault::ImpossibleState);
                            return Err(OneShotError::Backend(BackendFault::ImpossibleState));
                        }
                    }
                }
                return Ok(required);
            }
            // Preserve exact legacy diagnostics on rejection. A false negative
            // is an implementation fault, not permission to write a result.
            return if let Err(error) = validate_and_measure(settings, input) {
                Err(error)
            } else {
                if let Some(backend) = config.backend {
                    vector::quarantine(backend, BackendFault::ImpossibleState);
                }
                Err(OneShotError::Backend(BackendFault::ImpossibleState))
            };
        }
        validate_and_measure(settings, input)
    })
    .map_err(map_preflight_error)
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

pub(crate) fn write(
    proof: Preflight<'_, Prepared>,
    output: &mut [u8],
) -> Result<usize, OneShotError> {
    #[cfg(test)]
    crate::decode_backend::record_test_execution(crate::decode_backend::DecodeBackend::Scalar);
    if proof.len() == 0 {
        return Ok(0);
    }
    proof
        .write(
            output,
            |config, interior, tail, body_output, tail_output| {
                let settings = config.settings;
                if let Some(family) = Family::for_settings(settings) {
                    let table = family.table();
                    let read = config.backend.map_or(0, |backend| {
                        vector::write(backend, family, interior, body_output)
                    });
                    write_parts(
                        &interior[read..],
                        tail,
                        &mut body_output[read / 4 * 3..],
                        tail_output,
                        |byte| table[usize::from(byte)],
                    );
                } else {
                    write_parts(interior, tail, body_output, tail_output, |byte| {
                        value(settings, byte)
                    });
                }
            },
        )
        .map_err(|error| OneShotError::OutputTooSmall {
            required: error.required,
            available: error.available,
        })
}

fn write_parts(
    interior: &[u8],
    tail: &[u8],
    body_output: &mut [u8],
    tail_output: &mut [u8],
    value: impl Fn(u8) -> u8,
) {
    for (input, output) in interior
        .as_chunks::<4>()
        .0
        .iter()
        .zip(body_output.as_chunks_mut::<3>().0.iter_mut())
    {
        let first = value(input[0]);
        let second = value(input[1]);
        let third = value(input[2]);
        output[0] = (first << 2) | (second >> 4);
        output[1] = (second << 4) | (third >> 2);
        output[2] = (third << 6) | value(input[3]);
    }
    if !tail_output.is_empty() {
        let first = value(tail[0]);
        let second = value(tail[1]);
        tail_output[0] = (first << 2) | (second >> 4);
        if tail_output.len() > 1 {
            let third = value(tail[2]);
            tail_output[1] = (second << 4) | (third >> 2);
            if tail_output.len() > 2 {
                tail_output[2] = (third << 6) | value(tail[3]);
            }
        }
    }
}

fn value(settings: CodecSettings, byte: u8) -> u8 {
    settings.alphabet().decode_byte(byte).unwrap_or(0)
}

#[cfg(test)]
mod tests;

#[cfg(all(
    test,
    feature = "std",
    feature = "simd",
    any(target_arch = "x86", target_arch = "x86_64")
))]
pub(crate) mod ssse3_candidate;

#[cfg(all(
    test,
    feature = "std",
    feature = "simd",
    any(target_arch = "x86", target_arch = "x86_64")
))]
pub(crate) mod avx2_candidate;

#[cfg(all(
    test,
    feature = "std",
    feature = "simd",
    any(target_arch = "x86", target_arch = "x86_64")
))]
pub(crate) mod avx512_candidate;
