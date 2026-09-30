//! Test-only routing. Public dispatch is unchanged until vector admission.

use super::*;

fn candidate_len(settings: CodecSettings, input: &[u8], url_safe: bool) -> Option<usize> {
    // Keep the final quantum (including any padding) outside vector blocks.
    let prefix = input.len().saturating_sub(1) / 16 * 16;
    for block in input[..prefix].as_chunks::<16>().0 {
        if !crate::simd::candidate_validate_16(block, url_safe) {
            return None;
        }
    }
    let tail = validate_and_measure(settings, &input[prefix..]).ok()?;
    (prefix / 4).checked_mul(3)?.checked_add(tail)
}

#[cfg(test)]
pub(crate) fn decode(
    settings: CodecSettings,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, OneShotError> {
    let Some(family) = Family::for_settings(settings)
        .filter(|_| crate::simd::ssse3_validation_candidate_available())
    else {
        return prepare(settings, input, DecodeValidation::ScalarReference)
            .and_then(|proof| write(proof, output));
    };
    let url_safe = family == Family::UrlSafe;
    let proof = Preflight::validate(input, settings, |settings, input| {
        if let Some(required) = candidate_len(settings, input, url_safe) {
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
            let blocks = interior.len() / 16;
            for (input, output) in interior[..blocks * 16]
                .as_chunks::<16>()
                .0
                .iter()
                .zip(body_output[..blocks * 12].as_chunks_mut::<12>().0)
            {
                // This test-only assertion detects classifier/kernel disagreement.
                // Production admission must add backend fault/quarantine handling.
                assert!(crate::simd::candidate_decode_16(input, output, url_safe));
            }
            let table = family.table();
            write_parts(
                &interior[blocks * 16..],
                tail,
                &mut body_output[blocks * 12..],
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
mod tests;
