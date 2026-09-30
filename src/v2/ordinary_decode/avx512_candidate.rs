//! Test-only AVX-512 route; automatic and exact/static public policies are unchanged.
use super::*;

fn candidate_len(settings: CodecSettings, input: &[u8], url_safe: bool) -> Option<usize> {
    let prefix = input.len().saturating_sub(1) / 64 * 64;
    if !crate::simd::candidate_validate_avx512(&input[..prefix], url_safe) {
        return None;
    }
    // Reserve the final 1-64 bytes for original padding/canonical-tail validation.
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
        .filter(|_| crate::simd::avx512_validation_candidate_available())
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
            let blocks = interior.len() / 64;
            // Test-only invariant, not production fault handling. Quarantine and
            // recovery must be integrated before this candidate is admitted.
            assert!(crate::simd::candidate_decode_avx512(
                &interior[..blocks * 64],
                &mut body_output[..blocks * 48],
                url_safe
            ));
            let table = family.table();
            write_parts(
                &interior[blocks * 64..],
                tail,
                &mut body_output[blocks * 48..],
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
