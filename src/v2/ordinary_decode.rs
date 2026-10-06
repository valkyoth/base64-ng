//! Ordinary preflight and the private canonical writer.

use super::{
    contracts::{BackendFault, Status},
    incremental_decoder::DecoderState,
    ordinary::{OneShotError, map_operation_error},
    ordinary_scalar::Family,
    specifications::{CodecSettings, DecodePadding},
};
use crate::{
    DecodeReport, DecodeValidation, DecodeValidator,
    decode_preflight::{Failure as PreflightFailure, Preflight},
};

pub(crate) mod in_place;
pub(super) mod retained;
mod vector;

#[derive(Clone, Copy)]
pub(crate) enum Selection {
    Automatic,
    #[cfg(feature = "simd")]
    Static(Option<crate::runtime::Backend>),
}

#[derive(Clone, Copy)]
pub(crate) struct Prepared {
    settings: CodecSettings,
    backend: Option<crate::runtime::Backend>,
    // Bound to the same immutable settings as validation; avoid reclassification.
    family: Option<Family>,
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
    prepare_selected::<false, _>(settings, input, validation, None, validate_and_measure)
        .map_err(map_preflight_error)
}

// Historical callers recover their own errors and destination contracts. Do
// not compute canonical state-machine diagnostics just to discard them.
pub(crate) fn prepare_historical(
    settings: CodecSettings,
    input: &[u8],
) -> Option<Preflight<'_, Prepared>> {
    let family = Family::for_settings(settings)?;
    let padded = settings.decode_padding() == DecodePadding::RequireCanonical;
    prepare_selected::<false, _>(
        settings,
        input,
        DecodeValidation::Auto,
        None,
        |_, input| match (family, padded) {
            (Family::Standard, true) => crate::validate_decode::<crate::Standard, true>(input),
            (Family::Standard, false) => crate::validate_decode::<crate::Standard, false>(input),
            (Family::UrlSafe, true) => crate::validate_decode::<crate::UrlSafe, true>(input),
            (Family::UrlSafe, false) => crate::validate_decode::<crate::UrlSafe, false>(input),
        },
    )
    .ok()
}

#[inline]
fn prepare_selected<const STATIC: bool, E>(
    settings: CodecSettings,
    input: &[u8],
    validation: DecodeValidation,
    selected: Option<crate::runtime::Backend>,
    reference: impl Fn(CodecSettings, &[u8]) -> Result<usize, E>,
) -> Result<Preflight<'_, Prepared>, PreflightFailure<E>> {
    // Empty ordinary input needs no alphabet inspection or backend selection.
    // Preserve an explicit reference request on the reference validator.
    if validation == DecodeValidation::Auto && input.is_empty() {
        return Preflight::validate(
            input,
            Prepared {
                settings,
                backend: None,
                family: None,
            },
            |_, _| Ok(0),
        );
    }
    let family = if input.is_empty() {
        None
    } else {
        Family::for_settings(settings)
    };
    let backend = family.and_then(|_| {
        if STATIC {
            selected
        } else {
            vector::select(input.len())
        }
    });
    let config = Prepared {
        settings,
        backend,
        family,
    };
    Preflight::validate(input, config, |config, input| {
        let settings = config.settings;
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
                    match reference(settings, input) {
                        Ok(reference) if reference == required => {}
                        _ => {
                            vector::quarantine(backend, BackendFault::ImpossibleState);
                            return Err(PreflightFailure::ClassifierDisagreement);
                        }
                    }
                }
                return Ok(required);
            }
            // Preserve surface-specific diagnostics on rejection. A false negative
            // is an implementation fault, not permission to write a result.
            return if let Err(error) = reference(settings, input) {
                Err(PreflightFailure::Input(error))
            } else {
                if let Some(backend) = config.backend {
                    vector::quarantine(backend, BackendFault::ImpossibleState);
                }
                Err(PreflightFailure::ClassifierDisagreement)
            };
        }
        reference(settings, input).map_err(PreflightFailure::Input)
    })
    .map_err(|error| match error {
        PreflightFailure::Input(error) => error,
        PreflightFailure::Bounds => PreflightFailure::Bounds,
        PreflightFailure::ClassifierDisagreement => PreflightFailure::ClassifierDisagreement,
    })
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
    {
        crate::decode_validation::observation::record();
        crate::decode_validation::observation::record_canonical();
    }
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
            .update_reference(&input[input_offset..], &mut scratch)
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
    write_observed(proof, output, ())
}

#[inline]
fn write_observed<R: crate::decode_report::WriteObservation>(
    proof: Preflight<'_, Prepared>,
    output: &mut [u8],
    mut report: R,
) -> Result<usize, OneShotError> {
    #[cfg(test)]
    crate::decode_backend::record_test_execution(crate::decode_backend::DecodeBackend::Scalar);
    if proof.len() == 0 {
        return Ok(0);
    }
    proof
        .write(
            output,
            move |config, interior, tail, body_output, tail_output| {
                let settings = config.settings;
                if let Some(family) = config.family {
                    let table = family.table();
                    let read = config.backend.map_or(0, |backend| {
                        vector::write(backend, family, interior, body_output, &mut report)
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

pub(crate) fn decode_reported(
    settings: CodecSettings,
    input: &[u8],
    output: &mut [u8],
    validation: DecodeValidation,
    selected: Selection,
) -> Result<(usize, DecodeReport), OneShotError> {
    let mut report = DecodeReport::new(validation);
    let proof = match selected {
        Selection::Automatic => prepare(settings, input, validation)?,
        #[cfg(feature = "simd")]
        Selection::Static(backend) => {
            prepare_selected::<true, _>(settings, input, validation, backend, validate_and_measure)
                .map_err(map_preflight_error)?
        }
    };
    // Record the writer-eligible backend selected before validation.
    // ScalarReference does not execute it; validator() describes validation.
    // Do not re-probe health to reconstruct a possibly different selection.
    report.selected_backend = proof.configuration().backend;
    report.validator = if validation == DecodeValidation::ScalarReference {
        DecodeValidator::ScalarReference
    } else if input.is_empty() {
        DecodeValidator::Empty
    } else if proof.configuration().family.is_some() {
        report
            .selected_backend
            .map_or(DecodeValidator::ScalarTable, DecodeValidator::Vector)
    } else {
        DecodeValidator::ScalarReference
    };
    report.checked_validation =
        cfg!(feature = "checked-backend") && matches!(report.validator, DecodeValidator::Vector(_));
    let len = write_observed(proof, output, &mut report)?;
    Ok((len, report))
}

#[cfg(feature = "simd")]
pub(crate) fn static_backend(
    token: &crate::StaticBackendToken,
    len: usize,
) -> Option<crate::runtime::Backend> {
    // Only existing, safely available ordinary kernels may execute here. In
    // particular an AVX-512 token must not silently select a different ISA.
    (token.is_valid() && vector::eligible_static(token.backend(), len)).then_some(token.backend())
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

#[cfg(all(
    test,
    feature = "std",
    feature = "simd",
    target_arch = "aarch64",
    target_endian = "little"
))]
pub(crate) mod neon_candidate;
