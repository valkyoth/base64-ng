use crate::{
    PasslibPbkdf2Algorithm, PasslibPbkdf2Record, PasswordRecordError, PasswordRecordErrorKind,
    PasswordRecordLimits, error::map_base64,
};

const ADAPTED_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./";

/// Encodes one Passlib adapted-Base64 field transactionally.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for finite-limit, arithmetic, or capacity
/// failure.
pub fn encode_pbkdf2_field_into(
    input: &[u8],
    output: &mut [u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    require_work(input.len(), limits)?;
    if input.len() > limits.max_decoded_output_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    let required = encoded_len(input.len())?;
    require_field(required, limits)?;
    require_capacity(required, output.len())?;
    encode_adapted(input, &mut output[..required]);
    Ok(required)
}

/// Decodes one canonical Passlib adapted-Base64 field transactionally.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for malformed, noncanonical, finite-limit,
/// arithmetic, backend, or capacity failure.
pub fn decode_pbkdf2_field_into(
    input: &[u8],
    output: &mut [u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    require_work(input.len(), limits)?;
    require_field(input.len(), limits)?;
    let required = base64_ng::PBKDF2_ALPHABET_NO_PAD
        .decoded_len(input)
        .map_err(|error| map_base64(error, PasswordRecordErrorKind::InvalidField))?;
    if required > limits.max_decoded_output_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    require_capacity(required, output.len())?;
    base64_ng::PBKDF2_ALPHABET_NO_PAD
        .decode_into(input, &mut output[..required])
        .map_err(|error| map_base64(error, PasswordRecordErrorKind::InvalidField))
}

/// Parses and validates one exact Passlib PBKDF2 record.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for an unsupported identifier, malformed
/// structure, noncanonical rounds or fields, wrong checksum length, or limit
/// failure.
pub fn parse_pbkdf2_record(
    record: &[u8],
    limits: PasswordRecordLimits,
) -> Result<PasslibPbkdf2Record<'_>, PasswordRecordError> {
    require_record(record.len(), limits)?;
    let mut fields = record.split(|byte| *byte == b'$');
    if fields.next() != Some(&b""[..]) {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidStructure,
        ));
    }
    let algorithm = match fields.next() {
        Some(b"pbkdf2") => PasslibPbkdf2Algorithm::Sha1,
        Some(b"pbkdf2-sha256") => PasslibPbkdf2Algorithm::Sha256,
        Some(b"pbkdf2-sha512") => PasslibPbkdf2Algorithm::Sha512,
        _ => {
            return Err(PasswordRecordError::new(
                PasswordRecordErrorKind::InvalidPrefix,
            ));
        }
    };
    let rounds = fields
        .next()
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidStructure))?;
    let salt = fields
        .next()
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidStructure))?;
    let checksum = fields
        .next()
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidStructure))?;
    if fields.next().is_some() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidStructure,
        ));
    }

    require_field(salt.len(), limits)?;
    require_field(checksum.len(), limits)?;
    let rounds = parse_decimal(rounds, 1, u32::MAX)?;
    validate_pbkdf2_salt(salt, limits)?;
    validate_pbkdf2_checksum(algorithm, checksum, limits)?;
    Ok(PasslibPbkdf2Record {
        algorithm,
        rounds,
        salt,
        checksum,
    })
}

/// Validates PBKDF2 record components and returns the exact generated length.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for invalid rounds or checksum length,
/// finite-limit failure, or arithmetic overflow.
pub fn pbkdf2_record_len(
    algorithm: PasslibPbkdf2Algorithm,
    rounds: u32,
    salt: &[u8],
    checksum: &[u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    if rounds == 0 {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidRounds,
        ));
    }
    if salt.len() > limits.max_decoded_salt_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidSalt,
        ));
    }
    if checksum.len() != algorithm.checksum_len() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidChecksum,
        ));
    }
    let work = salt
        .len()
        .checked_add(checksum.len())
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::LengthOverflow))?;
    require_work(work, limits)?;
    if checksum.len() > limits.max_decoded_output_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    let salt_len = encoded_len(salt.len())?;
    let checksum_len = algorithm.encoded_checksum_len();
    require_field(salt_len, limits)?;
    require_field(checksum_len, limits)?;
    let required = algorithm
        .prefix()
        .len()
        .checked_add(decimal_len(rounds))
        .and_then(|length| length.checked_add(1))
        .and_then(|length| length.checked_add(salt_len))
        .and_then(|length| length.checked_add(1))
        .and_then(|length| length.checked_add(checksum_len))
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::LengthOverflow))?;
    require_generated(required, limits)?;
    Ok(required)
}

/// Generates one canonical Passlib PBKDF2 record transactionally.
///
/// `checksum` must contain the already-derived 20, 32, or 64 bytes required
/// by the selected format. This function performs no password hashing.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for invalid rounds or checksum length,
/// finite-limit failure, arithmetic overflow, or insufficient capacity.
pub fn generate_pbkdf2_record_into(
    algorithm: PasslibPbkdf2Algorithm,
    rounds: u32,
    salt: &[u8],
    checksum: &[u8],
    output: &mut [u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    let required = pbkdf2_record_len(algorithm, rounds, salt, checksum, limits)?;
    let salt_len = encoded_len(salt.len())?;
    require_capacity(required, output.len())?;

    let mut cursor = 0;
    cursor += write_bytes(&mut output[cursor..required], algorithm.prefix());
    cursor += write_decimal(rounds, &mut output[cursor..required]);
    output[cursor] = b'$';
    cursor += 1;
    encode_adapted(salt, &mut output[cursor..cursor + salt_len]);
    cursor += salt_len;
    output[cursor] = b'$';
    cursor += 1;
    encode_adapted(checksum, &mut output[cursor..required]);
    Ok(required)
}

fn validate_pbkdf2_salt(
    salt: &[u8],
    limits: PasswordRecordLimits,
) -> Result<(), PasswordRecordError> {
    let decoded = base64_ng::PBKDF2_ALPHABET_NO_PAD
        .decoded_len(salt)
        .map_err(|error| map_base64(error, PasswordRecordErrorKind::InvalidSalt))?;
    if decoded > limits.max_decoded_salt_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidSalt,
        ));
    }
    Ok(())
}

fn validate_pbkdf2_checksum(
    algorithm: PasslibPbkdf2Algorithm,
    checksum: &[u8],
    limits: PasswordRecordLimits,
) -> Result<(), PasswordRecordError> {
    if checksum.len() != algorithm.encoded_checksum_len() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidChecksum,
        ));
    }
    let decoded = base64_ng::PBKDF2_ALPHABET_NO_PAD
        .decoded_len(checksum)
        .map_err(|error| map_base64(error, PasswordRecordErrorKind::InvalidChecksum))?;
    if decoded != algorithm.checksum_len() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidChecksum,
        ));
    }
    if decoded > limits.max_decoded_output_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    Ok(())
}

pub(crate) fn parse_decimal(
    field: &[u8],
    minimum: u32,
    maximum: u32,
) -> Result<u32, PasswordRecordError> {
    if field.is_empty() || (field.len() > 1 && field[0] == b'0') {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidRounds,
        ));
    }
    let mut value = 0_u32;
    for (index, byte) in field.iter().copied().enumerate() {
        if !byte.is_ascii_digit() {
            return Err(PasswordRecordError::at(
                PasswordRecordErrorKind::InvalidRounds,
                index,
            ));
        }
        value = value
            .checked_mul(10)
            .and_then(|current| current.checked_add(u32::from(byte - b'0')))
            .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidRounds))?;
    }
    if value < minimum || value > maximum {
        Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidRounds,
        ))
    } else {
        Ok(value)
    }
}

pub(crate) const fn decimal_len(mut value: u32) -> usize {
    let mut length = 1;
    while value >= 10 {
        value /= 10;
        length += 1;
    }
    length
}

pub(crate) fn write_decimal(mut value: u32, output: &mut [u8]) -> usize {
    let length = decimal_len(value);
    let mut index = length;
    while index != 0 {
        index -= 1;
        output[index] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
    }
    length
}

pub(crate) fn write_bytes(output: &mut [u8], input: &[u8]) -> usize {
    output[..input.len()].copy_from_slice(input);
    input.len()
}

pub(crate) fn require_record(
    length: usize,
    limits: PasswordRecordLimits,
) -> Result<(), PasswordRecordError> {
    if length > limits.max_record_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InputLimitExceeded,
        ));
    }
    require_work(length, limits)
}

pub(crate) fn require_work(
    length: usize,
    limits: PasswordRecordLimits,
) -> Result<(), PasswordRecordError> {
    if length > limits.max_work_before_output() {
        Err(PasswordRecordError::new(
            PasswordRecordErrorKind::WorkLimitExceeded,
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn require_field(
    length: usize,
    limits: PasswordRecordLimits,
) -> Result<(), PasswordRecordError> {
    if length > limits.max_field_bytes() {
        Err(PasswordRecordError::new(
            PasswordRecordErrorKind::FieldLimitExceeded,
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn require_generated(
    length: usize,
    limits: PasswordRecordLimits,
) -> Result<(), PasswordRecordError> {
    if length > limits.max_generated_bytes() {
        Err(PasswordRecordError::new(
            PasswordRecordErrorKind::OutputLimitExceeded,
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn require_capacity(
    required: usize,
    available: usize,
) -> Result<(), PasswordRecordError> {
    if required > available {
        Err(PasswordRecordError::capacity(required, available))
    } else {
        Ok(())
    }
}

fn encoded_len(length: usize) -> Result<usize, PasswordRecordError> {
    let full = length / 3;
    let tail = length % 3;
    full.checked_mul(4)
        .and_then(|value| value.checked_add(if tail == 0 { 0 } else { tail + 1 }))
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::LengthOverflow))
}

fn encode_adapted(input: &[u8], output: &mut [u8]) {
    let mut source = 0;
    let mut destination = 0;
    while source + 3 <= input.len() {
        let bits = (u32::from(input[source]) << 16)
            | (u32::from(input[source + 1]) << 8)
            | u32::from(input[source + 2]);
        output[destination] = ADAPTED_ALPHABET[((bits >> 18) & 0x3f) as usize];
        output[destination + 1] = ADAPTED_ALPHABET[((bits >> 12) & 0x3f) as usize];
        output[destination + 2] = ADAPTED_ALPHABET[((bits >> 6) & 0x3f) as usize];
        output[destination + 3] = ADAPTED_ALPHABET[(bits & 0x3f) as usize];
        source += 3;
        destination += 4;
    }
    let remaining = input.len() - source;
    if remaining == 1 {
        let bits = u16::from(input[source]) << 4;
        output[destination] = ADAPTED_ALPHABET[((bits >> 6) & 0x3f) as usize];
        output[destination + 1] = ADAPTED_ALPHABET[(bits & 0x3f) as usize];
    } else if remaining == 2 {
        let bits = (u32::from(input[source]) << 10) | (u32::from(input[source + 1]) << 2);
        output[destination] = ADAPTED_ALPHABET[((bits >> 12) & 0x3f) as usize];
        output[destination + 1] = ADAPTED_ALPHABET[((bits >> 6) & 0x3f) as usize];
        output[destination + 2] = ADAPTED_ALPHABET[(bits & 0x3f) as usize];
    }
}
