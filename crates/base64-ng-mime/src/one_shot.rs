use crate::{
    MimeBodyDecodePolicy, MimeBodyDecodeReport, MimeBodyDecoder, MimeBodyError, MimeBodyErrorKind,
    MimeBodyLimits, MimeBodyStatus, MimeBodyTerminalLineEnding,
};

const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Returns the exact canonical RFC 2045 content-transfer body length.
///
/// # Errors
///
/// Returns [`MimeBodyError`] when the exact length overflows `usize`.
pub fn mime_content_transfer_body_encoded_len(
    input_len: usize,
    terminal: MimeBodyTerminalLineEnding,
) -> Result<usize, MimeBodyError> {
    let complete = input_len / 3;
    let tail = usize::from(!input_len.is_multiple_of(3));
    let quanta = complete
        .checked_add(tail)
        .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?;
    let encoded = quanta
        .checked_mul(4)
        .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?;
    if encoded == 0 {
        return Ok(0);
    }
    let separators = (encoded - 1) / 76;
    let terminal_break = usize::from(matches!(terminal, MimeBodyTerminalLineEnding::IncludeCrLf));
    let line_breaks = separators
        .checked_add(terminal_break)
        .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?;
    encoded
        .checked_add(
            line_breaks
                .checked_mul(2)
                .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?,
        )
        .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))
}

/// Encodes one body transactionally into caller-owned storage.
///
/// # Errors
///
/// Returns [`MimeBodyError`] for finite-limit failure, insufficient output,
/// length overflow, or an internal invariant failure.
pub fn encode_mime_content_transfer_body_into(
    input: &[u8],
    output: &mut [u8],
    limits: MimeBodyLimits,
    terminal: MimeBodyTerminalLineEnding,
) -> Result<usize, MimeBodyError> {
    if input.len() > limits.max_input_bytes() {
        return Err(MimeBodyError::new(MimeBodyErrorKind::InputLimitExceeded));
    }
    let required = mime_content_transfer_body_encoded_len(input.len(), terminal)?;
    let encoded_payload = base64_ng::STRICT_STANDARD_PADDED
        .encoded_len(input.len())
        .map_err(crate::error::map_base64)?;
    if encoded_payload.min(76) > limits.max_physical_line_bytes() {
        return Err(MimeBodyError::new(MimeBodyErrorKind::PhysicalLineTooLong));
    }
    if required > limits.max_encoded_output_bytes() {
        return Err(MimeBodyError::new(MimeBodyErrorKind::OutputLimitExceeded));
    }
    if output.len() < required {
        return Err(MimeBodyError::capacity(required, output.len()));
    }

    Ok(encode_validated_body(
        input,
        &mut output[..required],
        terminal,
    ))
}

fn encode_validated_body(
    input: &[u8],
    output: &mut [u8],
    terminal: MimeBodyTerminalLineEnding,
) -> usize {
    let mut read = 0;
    let mut write = 0;
    let mut column = 0;
    while read + 3 <= input.len() {
        insert_line_break(output, &mut write, &mut column);
        encode_three(&input[read..read + 3], &mut output[write..write + 4]);
        read += 3;
        write += 4;
        column += 4;
    }
    if read != input.len() {
        insert_line_break(output, &mut write, &mut column);
        encode_tail(&input[read..], &mut output[write..write + 4]);
        write += 4;
        column += 4;
    }
    if column != 0 && matches!(terminal, MimeBodyTerminalLineEnding::IncludeCrLf) {
        output[write..write + 2].copy_from_slice(b"\r\n");
        write += 2;
    }
    write
}

fn insert_line_break(output: &mut [u8], write: &mut usize, column: &mut usize) {
    if *column == 76 {
        output[*write..*write + 2].copy_from_slice(b"\r\n");
        *write += 2;
        *column = 0;
    }
}

fn encode_three(input: &[u8], output: &mut [u8]) {
    output[0] = STANDARD_ALPHABET[usize::from(input[0] >> 2)];
    output[1] = STANDARD_ALPHABET[usize::from(((input[0] & 3) << 4) | (input[1] >> 4))];
    output[2] = STANDARD_ALPHABET[usize::from(((input[1] & 15) << 2) | (input[2] >> 6))];
    output[3] = STANDARD_ALPHABET[usize::from(input[2] & 63)];
}

fn encode_tail(input: &[u8], output: &mut [u8]) {
    output[0] = STANDARD_ALPHABET[usize::from(input[0] >> 2)];
    output[1] = STANDARD_ALPHABET[usize::from((input[0] & 3) << 4)];
    if input.len() == 1 {
        output[2..4].copy_from_slice(b"==");
    } else {
        output[1] = STANDARD_ALPHABET[usize::from(((input[0] & 3) << 4) | (input[1] >> 4))];
        output[2] = STANDARD_ALPHABET[usize::from((input[1] & 15) << 2)];
        output[3] = b'=';
    }
}

/// Validates and measures one RFC 2045 content-transfer body.
///
/// # Errors
///
/// Returns [`MimeBodyError`] for malformed input or finite-limit failure.
pub fn validate_mime_content_transfer_body(
    input: &[u8],
    policy: MimeBodyDecodePolicy,
    limits: MimeBodyLimits,
) -> Result<MimeBodyDecodeReport, MimeBodyError> {
    drive_decode(input, &mut [], policy, limits, true)
}

/// Decodes one body transactionally into caller-owned storage.
///
/// A complete validation pass runs before the first destination write. Every
/// returned error therefore leaves `output` unchanged.
///
/// # Errors
///
/// Returns [`MimeBodyError`] for malformed input, finite-limit failure,
/// insufficient output, or an internal invariant failure.
pub fn decode_mime_content_transfer_body_into(
    input: &[u8],
    output: &mut [u8],
    policy: MimeBodyDecodePolicy,
    limits: MimeBodyLimits,
) -> Result<MimeBodyDecodeReport, MimeBodyError> {
    let measured = validate_mime_content_transfer_body(input, policy, limits)?;
    if output.len() < measured.output_bytes() {
        return Err(MimeBodyError::capacity(
            measured.output_bytes(),
            output.len(),
        ));
    }
    let report = drive_decode(
        input,
        &mut output[..measured.output_bytes()],
        policy,
        limits,
        false,
    )?;
    if report != measured {
        return Err(MimeBodyError::new(MimeBodyErrorKind::InternalInvariant));
    }
    Ok(report)
}

fn drive_decode(
    input: &[u8],
    output: &mut [u8],
    policy: MimeBodyDecodePolicy,
    limits: MimeBodyLimits,
    measure_only: bool,
) -> Result<MimeBodyDecodeReport, MimeBodyError> {
    let mut decoder = MimeBodyDecoder::new(policy, limits);
    let mut input_offset = 0;
    let mut output_offset = 0;
    let mut scratch = [0u8; 3];
    while input_offset < input.len() {
        let destination = if measure_only {
            &mut scratch[..]
        } else {
            &mut output[output_offset..]
        };
        let step = decoder.update(&input[input_offset..], destination)?;
        input_offset += step.progress().input_consumed();
        if !measure_only {
            output_offset += step.progress().output_produced();
        }
        if step.progress().input_consumed() == 0 && step.progress().output_produced() == 0 {
            return Err(MimeBodyError::new(MimeBodyErrorKind::InternalInvariant));
        }
    }
    loop {
        let destination = if measure_only {
            &mut scratch[..]
        } else {
            &mut output[output_offset..]
        };
        let step = decoder.finish(destination)?;
        if !measure_only {
            output_offset += step.progress().output_produced();
        }
        if step.status() == MimeBodyStatus::Complete {
            return Ok(decoder.mime_body_decode_report());
        }
        if step.progress().output_produced() == 0 {
            return Err(MimeBodyError::new(MimeBodyErrorKind::InternalInvariant));
        }
    }
}

#[cfg(feature = "alloc")]
mod allocating {
    use alloc::{string::String, vec::Vec};

    use super::{
        MimeBodyDecodePolicy, MimeBodyDecodeReport, MimeBodyError, MimeBodyErrorKind,
        MimeBodyLimits, MimeBodyTerminalLineEnding, decode_mime_content_transfer_body_into,
        encode_mime_content_transfer_body_into, mime_content_transfer_body_encoded_len,
        validate_mime_content_transfer_body,
    };

    /// Encodes into a fallibly allocated canonical body string.
    ///
    /// # Errors
    ///
    /// Returns [`MimeBodyError`] for finite-limit, length, allocation, or
    /// internal invariant failures.
    pub fn encode_mime_content_transfer_body_to_string(
        input: &[u8],
        limits: MimeBodyLimits,
        terminal: MimeBodyTerminalLineEnding,
    ) -> Result<String, MimeBodyError> {
        if input.len() > limits.max_input_bytes() {
            return Err(MimeBodyError::new(MimeBodyErrorKind::InputLimitExceeded));
        }
        let required = mime_content_transfer_body_encoded_len(input.len(), terminal)?;
        if required > limits.max_encoded_output_bytes() {
            return Err(MimeBodyError::new(MimeBodyErrorKind::OutputLimitExceeded));
        }
        let encoded_payload = base64_ng::STRICT_STANDARD_PADDED
            .encoded_len(input.len())
            .map_err(crate::error::map_base64)?;
        if encoded_payload.min(76) > limits.max_physical_line_bytes() {
            return Err(MimeBodyError::new(MimeBodyErrorKind::PhysicalLineTooLong));
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(required)
            .map_err(|_| MimeBodyError::new(MimeBodyErrorKind::AllocationFailed))?;
        output.resize(required, 0);
        encode_mime_content_transfer_body_into(input, &mut output, limits, terminal)?;
        String::from_utf8(output)
            .map_err(|_| MimeBodyError::new(MimeBodyErrorKind::InternalInvariant))
    }

    /// Decodes into a fallibly allocated ordinary byte vector.
    ///
    /// # Errors
    ///
    /// Returns [`MimeBodyError`] for malformed input, finite-limit,
    /// allocation, or internal invariant failures.
    pub fn decode_mime_content_transfer_body_to_vec(
        input: &[u8],
        policy: MimeBodyDecodePolicy,
        limits: MimeBodyLimits,
    ) -> Result<(Vec<u8>, MimeBodyDecodeReport), MimeBodyError> {
        let report = validate_mime_content_transfer_body(input, policy, limits)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(report.output_bytes())
            .map_err(|_| MimeBodyError::new(MimeBodyErrorKind::AllocationFailed))?;
        output.resize(report.output_bytes(), 0);
        let decoded = decode_mime_content_transfer_body_into(input, &mut output, policy, limits)?;
        Ok((output, decoded))
    }
}

#[cfg(feature = "alloc")]
pub use allocating::{
    decode_mime_content_transfer_body_to_vec, encode_mime_content_transfer_body_to_string,
};
