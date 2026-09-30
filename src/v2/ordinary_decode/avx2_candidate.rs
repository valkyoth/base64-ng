//! Test-only AVX2 route: public dispatch and scalar-reference policy are unchanged.
use super::*;

fn candidate_len(settings: CodecSettings, input: &[u8], url_safe: bool) -> Option<usize> {
    let prefix = input.len().saturating_sub(1) / 32 * 32;
    if !crate::simd::candidate_validate_avx2(&input[..prefix], url_safe) {
        return None;
    }
    // Only the final 1-32 bytes (zero for empty input) need reference validation.
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
        .filter(|_| crate::simd::avx2_validation_candidate_available())
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
            let blocks = interior.len() / 32;
            // As for Commit 6, disagreement is a test failure. Production fault
            // recovery/quarantine must be integrated before admitting this route.
            assert!(crate::simd::candidate_decode_avx2(
                &interior[..blocks * 32],
                &mut body_output[..blocks * 24],
                url_safe
            ));
            let table = family.table();
            write_parts(
                &interior[blocks * 32..],
                tail,
                &mut body_output[blocks * 24..],
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
