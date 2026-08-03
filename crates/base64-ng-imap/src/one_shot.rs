use crate::{ImapPayloadError, ImapPayloadErrorKind, ImapPayloadLimits, error::map_one_shot};

/// Returns the exact modified-Base64 payload length for UTF-16BE octets.
///
/// # Errors
///
/// Returns [`ImapPayloadErrorKind::InvalidUtf16BeLength`] for an odd byte
/// length or [`ImapPayloadErrorKind::LengthOverflow`] for arithmetic overflow.
pub fn modified_utf7_payload_encoded_len(utf16be_len: usize) -> Result<usize, ImapPayloadError> {
    require_utf16be_length(utf16be_len)?;
    base64_ng::IMAP_MUTF7_ALPHABET_NO_PAD
        .encoded_len(utf16be_len)
        .map_err(map_one_shot)
}

/// Validates and returns the exact decoded UTF-16BE byte length.
///
/// # Errors
///
/// Returns [`ImapPayloadError`] for malformed, padded, noncanonical, odd-byte,
/// or finite-limit input.
pub fn modified_utf7_payload_decoded_len(
    payload: &[u8],
    limits: ImapPayloadLimits,
) -> Result<usize, ImapPayloadError> {
    preflight_input(payload.len(), limits)?;
    let required = base64_ng::IMAP_MUTF7_ALPHABET_NO_PAD
        .decoded_len(payload)
        .map_err(map_one_shot)?;
    require_utf16be_length(required)?;
    require_output_limit(required, limits)?;
    Ok(required)
}

/// Encodes already-converted UTF-16BE octets transactionally.
///
/// This emits payload text only. It does not emit modified UTF-7 shift
/// delimiters or decide whether a Unicode character should be shifted.
/// Every error leaves `output` unchanged.
///
/// # Errors
///
/// Returns [`ImapPayloadError`] for odd UTF-16BE storage, finite limits,
/// arithmetic/backend failure, or insufficient destination capacity.
pub fn encode_modified_utf7_payload_into(
    utf16be: &[u8],
    output: &mut [u8],
    limits: ImapPayloadLimits,
) -> Result<usize, ImapPayloadError> {
    preflight_input(utf16be.len(), limits)?;
    let required = modified_utf7_payload_encoded_len(utf16be.len())?;
    require_output_limit(required, limits)?;
    require_capacity(required, output.len())?;
    base64_ng::IMAP_MUTF7_ALPHABET_NO_PAD
        .encode_into(utf16be, &mut output[..required])
        .map_err(map_one_shot)
}

/// Validates one complete modified-Base64 payload.
///
/// # Errors
///
/// Returns [`ImapPayloadError`] for malformed, padded, noncanonical, odd-byte,
/// or finite-limit input.
pub fn validate_modified_utf7_payload(
    payload: &[u8],
    limits: ImapPayloadLimits,
) -> Result<(), ImapPayloadError> {
    modified_utf7_payload_decoded_len(payload, limits).map(|_| ())
}

/// Decodes one complete modified-Base64 payload transactionally.
///
/// The output is already-encoded UTF-16BE storage, not Unicode text. Every
/// error leaves `output` unchanged.
///
/// # Errors
///
/// Returns [`ImapPayloadError`] for malformed, padded, noncanonical, odd-byte,
/// or finite-limit input, backend failure, or insufficient capacity.
pub fn decode_modified_utf7_payload_into(
    payload: &[u8],
    output: &mut [u8],
    limits: ImapPayloadLimits,
) -> Result<usize, ImapPayloadError> {
    let required = modified_utf7_payload_decoded_len(payload, limits)?;
    require_capacity(required, output.len())?;
    base64_ng::IMAP_MUTF7_ALPHABET_NO_PAD
        .decode_into(payload, &mut output[..required])
        .map_err(map_one_shot)
}

pub(crate) fn preflight_input(
    input_len: usize,
    limits: ImapPayloadLimits,
) -> Result<(), ImapPayloadError> {
    if input_len > limits.max_input_bytes() {
        return Err(ImapPayloadError::new(
            ImapPayloadErrorKind::InputLimitExceeded,
        ));
    }
    if input_len > limits.max_work_before_output() {
        return Err(ImapPayloadError::new(
            ImapPayloadErrorKind::WorkLimitExceeded,
        ));
    }
    Ok(())
}

pub(crate) fn require_output_limit(
    required: usize,
    limits: ImapPayloadLimits,
) -> Result<(), ImapPayloadError> {
    if required > limits.max_output_bytes() {
        Err(ImapPayloadError::new(
            ImapPayloadErrorKind::OutputLimitExceeded,
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn require_capacity(required: usize, available: usize) -> Result<(), ImapPayloadError> {
    if required > available {
        Err(ImapPayloadError::capacity(required, available))
    } else {
        Ok(())
    }
}

pub(crate) fn require_utf16be_length(length: usize) -> Result<(), ImapPayloadError> {
    if length.is_multiple_of(2) {
        Ok(())
    } else {
        Err(ImapPayloadError::new(
            ImapPayloadErrorKind::InvalidUtf16BeLength,
        ))
    }
}
