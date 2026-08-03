use crate::{
    Base64MultibaseEncoding, Base64MultibaseError, Base64MultibaseErrorKind, Base64MultibaseLimits,
    DecodedBase64Multibase, error::map_one_shot,
};

/// Returns the exact prefixed encoded length.
///
/// # Errors
///
/// Returns [`Base64MultibaseErrorKind::LengthOverflow`] when the Base64 body
/// plus one prefix byte cannot be represented by `usize`.
pub fn base64_multibase_encoded_len(
    encoding: Base64MultibaseEncoding,
    input_len: usize,
) -> Result<usize, Base64MultibaseError> {
    encoding
        .codec()
        .encoded_len(input_len)
        .map_err(map_one_shot)?
        .checked_add(1)
        .ok_or_else(|| Base64MultibaseError::new(Base64MultibaseErrorKind::LengthOverflow))
}

/// Encodes one complete Base64-family multibase value transactionally.
///
/// Every error leaves the entire destination unchanged. Output begins with
/// the exact registry prefix selected by `encoding`.
///
/// # Errors
///
/// Returns [`Base64MultibaseError`] for finite limits, length overflow,
/// backend failure, or insufficient destination capacity.
pub fn encode_base64_multibase_into(
    encoding: Base64MultibaseEncoding,
    input: &[u8],
    output: &mut [u8],
    limits: Base64MultibaseLimits,
) -> Result<usize, Base64MultibaseError> {
    preflight_encode_input(input.len(), limits)?;
    let required = base64_multibase_encoded_len(encoding, input.len())?;
    require_output_limit(required, limits)?;
    require_capacity(required, output.len())?;
    encoding
        .codec()
        .encode_into(input, &mut output[1..required])
        .map_err(map_one_shot)?;
    output[0] = encoding.prefix();
    Ok(required)
}

/// Validates one complete Base64-family multibase value without decoding.
///
/// # Errors
///
/// Returns [`Base64MultibaseError`] for a missing/unsupported prefix,
/// noncanonical payload, or finite-limit failure.
pub fn validate_base64_multibase(
    input: &[u8],
    limits: Base64MultibaseLimits,
) -> Result<Base64MultibaseEncoding, Base64MultibaseError> {
    let (encoding, payload) = select_payload(input, limits)?;
    let required = encoding
        .codec()
        .decoded_len(payload)
        .map_err(map_one_shot)?;
    require_output_limit(required, limits)?;
    Ok(encoding)
}

/// Decodes one complete Base64-family multibase value transactionally.
///
/// Every error leaves the entire destination unchanged. The returned value
/// identifies the exact prefix and initialized output prefix.
///
/// # Errors
///
/// Returns [`Base64MultibaseError`] for a missing/unsupported prefix,
/// noncanonical payload, finite-limit failure, backend failure, or
/// insufficient destination capacity.
pub fn decode_base64_multibase_into(
    input: &[u8],
    output: &mut [u8],
    limits: Base64MultibaseLimits,
) -> Result<DecodedBase64Multibase, Base64MultibaseError> {
    let (encoding, payload) = select_payload(input, limits)?;
    let required = encoding
        .codec()
        .decoded_len(payload)
        .map_err(map_one_shot)?;
    require_output_limit(required, limits)?;
    require_capacity(required, output.len())?;
    let written = encoding
        .codec()
        .decode_into(payload, &mut output[..required])
        .map_err(map_one_shot)?;
    Ok(DecodedBase64Multibase::new(encoding, written))
}

pub(crate) fn select_payload(
    input: &[u8],
    limits: Base64MultibaseLimits,
) -> Result<(Base64MultibaseEncoding, &[u8]), Base64MultibaseError> {
    preflight_decode_input(input.len(), limits)?;
    let Some((&prefix, payload)) = input.split_first() else {
        return Err(Base64MultibaseError::new(
            Base64MultibaseErrorKind::MissingPrefix,
        ));
    };
    let Some(encoding) = Base64MultibaseEncoding::from_prefix(prefix) else {
        return Err(Base64MultibaseError::unsupported(prefix));
    };
    Ok((encoding, payload))
}

pub(crate) fn preflight_encode_input(
    input_len: usize,
    limits: Base64MultibaseLimits,
) -> Result<(), Base64MultibaseError> {
    if input_len > limits.max_input_bytes() {
        return Err(Base64MultibaseError::new(
            Base64MultibaseErrorKind::InputLimitExceeded,
        ));
    }
    if input_len > limits.max_work_before_output() {
        return Err(Base64MultibaseError::new(
            Base64MultibaseErrorKind::WorkLimitExceeded,
        ));
    }
    Ok(())
}

pub(crate) fn preflight_decode_input(
    input_len: usize,
    limits: Base64MultibaseLimits,
) -> Result<(), Base64MultibaseError> {
    preflight_encode_input(input_len, limits)
}

pub(crate) fn require_output_limit(
    required: usize,
    limits: Base64MultibaseLimits,
) -> Result<(), Base64MultibaseError> {
    if required > limits.max_output_bytes() {
        Err(Base64MultibaseError::new(
            Base64MultibaseErrorKind::OutputLimitExceeded,
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn require_capacity(
    required: usize,
    available: usize,
) -> Result<(), Base64MultibaseError> {
    if required > available {
        Err(Base64MultibaseError::capacity(required, available))
    } else {
        Ok(())
    }
}
