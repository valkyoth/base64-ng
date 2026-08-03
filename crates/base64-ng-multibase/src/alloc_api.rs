use alloc::{string::String, vec::Vec};

use crate::{
    Base64MultibaseEncoding, Base64MultibaseError, Base64MultibaseErrorKind, Base64MultibaseLimits,
    DecodedBase64MultibaseVec, base64_multibase_encoded_len, error::map_one_shot,
    one_shot::select_payload,
};

/// Encodes one Base64-family multibase value into a newly allocated string.
///
/// # Errors
///
/// Returns [`Base64MultibaseError`] for finite-limit, arithmetic, backend, or
/// allocation failure.
pub fn encode_base64_multibase_to_string(
    encoding: Base64MultibaseEncoding,
    input: &[u8],
    limits: Base64MultibaseLimits,
) -> Result<String, Base64MultibaseError> {
    crate::one_shot::preflight_encode_input(input.len(), limits)?;
    let required = base64_multibase_encoded_len(encoding, input.len())?;
    crate::one_shot::require_output_limit(required, limits)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| Base64MultibaseError::new(Base64MultibaseErrorKind::AllocationFailed))?;
    output.resize(required, 0);
    crate::encode_base64_multibase_into(encoding, input, &mut output, limits)?;
    String::from_utf8(output)
        .map_err(|_| Base64MultibaseError::new(Base64MultibaseErrorKind::BackendFailure))
}

/// Decodes one Base64-family multibase value into a newly allocated vector.
///
/// Validation and exact sizing complete before allocation or plaintext
/// materialization.
///
/// # Errors
///
/// Returns [`Base64MultibaseError`] for prefix, payload, finite-limit,
/// backend, or allocation failure.
pub fn decode_base64_multibase_to_vec(
    input: &[u8],
    limits: Base64MultibaseLimits,
) -> Result<DecodedBase64MultibaseVec, Base64MultibaseError> {
    let (encoding, payload) = select_payload(input, limits)?;
    let required = encoding
        .codec()
        .decoded_len(payload)
        .map_err(map_one_shot)?;
    crate::one_shot::require_output_limit(required, limits)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| Base64MultibaseError::new(Base64MultibaseErrorKind::AllocationFailed))?;
    output.resize(required, 0);
    encoding
        .codec()
        .decode_into(payload, &mut output)
        .map_err(map_one_shot)?;
    Ok(DecodedBase64MultibaseVec::new(encoding, output))
}
