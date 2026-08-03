use alloc::{string::String, vec::Vec};

use crate::{
    ImapPayloadError, ImapPayloadErrorKind, ImapPayloadLimits, decode_modified_utf7_payload_into,
    encode_modified_utf7_payload_into, modified_utf7_payload_decoded_len,
    modified_utf7_payload_encoded_len,
    one_shot::{preflight_input, require_output_limit},
};

/// Encodes UTF-16BE octets into an allocated modified-Base64 payload string.
///
/// # Errors
///
/// Returns [`ImapPayloadError`] for malformed input geometry, finite limits,
/// allocation, arithmetic, or backend failure.
pub fn encode_modified_utf7_payload_to_string(
    utf16be: &[u8],
    limits: ImapPayloadLimits,
) -> Result<String, ImapPayloadError> {
    preflight_input(utf16be.len(), limits)?;
    let required = modified_utf7_payload_encoded_len(utf16be.len())?;
    require_output_limit(required, limits)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| ImapPayloadError::new(ImapPayloadErrorKind::AllocationFailed))?;
    output.resize(required, 0);
    encode_modified_utf7_payload_into(utf16be, &mut output, limits)?;
    String::from_utf8(output)
        .map_err(|_| ImapPayloadError::new(ImapPayloadErrorKind::BackendFailure))
}

/// Decodes a modified-Base64 payload into allocated UTF-16BE octets.
///
/// Validation and exact sizing complete before allocation.
///
/// # Errors
///
/// Returns [`ImapPayloadError`] for malformed input, finite limits,
/// allocation, arithmetic, or backend failure.
pub fn decode_modified_utf7_payload_to_vec(
    payload: &[u8],
    limits: ImapPayloadLimits,
) -> Result<Vec<u8>, ImapPayloadError> {
    let required = modified_utf7_payload_decoded_len(payload, limits)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| ImapPayloadError::new(ImapPayloadErrorKind::AllocationFailed))?;
    output.resize(required, 0);
    decode_modified_utf7_payload_into(payload, &mut output, limits)?;
    Ok(output)
}
