//! Evaluation-only NEON path. Production routing and health checks are unchanged.
use super::*;
use crate::simd::ordinary as neon;

fn candidate_len(settings: CodecSettings, input: &[u8], family: Family) -> Option<usize> {
    let prefix = input.len().saturating_sub(1) / 16 * 16;
    if prefix != 0
        && !neon::validate(
            crate::runtime::Backend::Neon,
            &input[..prefix],
            family == Family::UrlSafe,
        )
    {
        return None;
    }
    let tail = family.validated_len(
        &input[prefix..],
        settings.decode_padding() == DecodePadding::RequireCanonical,
    )?;
    (prefix / 4).checked_mul(3)?.checked_add(tail)
}

#[cfg(test)]
pub(crate) fn decode(
    settings: CodecSettings,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, OneShotError> {
    let Some(family) = Family::for_settings(settings) else {
        return prepare(settings, input, DecodeValidation::ScalarReference)
            .and_then(|proof| write(proof, output));
    };
    let url_safe = family == Family::UrlSafe;
    let proof = Preflight::validate(input, settings, |settings, input| {
        if let Some(required) = candidate_len(settings, input, family) {
            return Ok(required);
        }
        match validate_and_measure(settings, input) {
            Err(error) => Err(error),
            Ok(_) => Err(OneShotError::Backend(BackendFault::ImpossibleState)),
        }
    })
    .map_err(map_preflight_error)?;
    proof
        .write(output, |_, interior, tail, body_output, tail_output| {
            let prefix = interior.len() / 16 * 16;
            let written = prefix / 4 * 3;
            // Evaluation only: disagreement must fail the test. Production admission
            // requires shared health KAT, quarantine/recovery and checked comparison.
            assert!(
                prefix == 0
                    || neon::decode(
                        crate::runtime::Backend::Neon,
                        &interior[..prefix],
                        &mut body_output[..written],
                        url_safe
                    )
            );
            let table = family.table();
            write_parts(
                &interior[prefix..],
                tail,
                &mut body_output[written..],
                tail_output,
                |byte| table[usize::from(byte)],
            );
        })
        .map_err(|error| OneShotError::OutputTooSmall {
            required: error.required,
            available: error.available,
        })
}

#[cfg(test)]
mod benchmark;
#[cfg(test)]
mod tests;
