use crate::{
    PasswordRecordError, PasswordRecordErrorKind, PasswordRecordLimits, ShaCryptAlgorithm,
    ShaCryptRecord, ShaCryptRounds,
    limits::WorkBudget,
    pbkdf2::{
        decimal_len, parse_decimal, require_capacity, require_field, require_generated,
        require_record, write_bytes, write_decimal,
    },
};

const CRYPT_ALPHABET: &[u8; 64] =
    b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const SHA256_GROUPS: &[(Option<usize>, Option<usize>, usize, usize)] = &[
    (Some(0), Some(10), 20, 4),
    (Some(21), Some(1), 11, 4),
    (Some(12), Some(22), 2, 4),
    (Some(3), Some(13), 23, 4),
    (Some(24), Some(4), 14, 4),
    (Some(15), Some(25), 5, 4),
    (Some(6), Some(16), 26, 4),
    (Some(27), Some(7), 17, 4),
    (Some(18), Some(28), 8, 4),
    (Some(9), Some(19), 29, 4),
    (None, Some(31), 30, 3),
];
const SHA512_GROUPS: &[(Option<usize>, Option<usize>, usize, usize)] = &[
    (Some(0), Some(21), 42, 4),
    (Some(22), Some(43), 1, 4),
    (Some(44), Some(2), 23, 4),
    (Some(3), Some(24), 45, 4),
    (Some(25), Some(46), 4, 4),
    (Some(47), Some(5), 26, 4),
    (Some(6), Some(27), 48, 4),
    (Some(28), Some(49), 7, 4),
    (Some(50), Some(8), 29, 4),
    (Some(9), Some(30), 51, 4),
    (Some(31), Some(52), 10, 4),
    (Some(53), Some(11), 32, 4),
    (Some(12), Some(33), 54, 4),
    (Some(34), Some(55), 13, 4),
    (Some(56), Some(14), 35, 4),
    (Some(15), Some(36), 57, 4),
    (Some(37), Some(58), 16, 4),
    (Some(59), Some(17), 38, 4),
    (Some(18), Some(39), 60, 4),
    (Some(40), Some(61), 19, 4),
    (Some(62), Some(20), 41, 4),
    (None, None, 63, 2),
];

/// Encodes a raw SHA-crypt digest using the format-specific permutation.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for wrong digest length, finite-limit
/// failure, or insufficient capacity.
pub fn encode_sha_crypt_checksum_into(
    algorithm: ShaCryptAlgorithm,
    digest: &[u8],
    output: &mut [u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    if digest.len() != algorithm.digest_len() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidChecksum,
        ));
    }
    let mut work = WorkBudget::new(limits);
    work.charge(digest.len())?;
    if digest.len() > limits.max_decoded_output_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    let required = algorithm.encoded_checksum_len();
    require_field(required, limits)?;
    require_capacity(required, output.len())?;
    encode_groups(digest, groups(algorithm), &mut output[..required]);
    Ok(required)
}

/// Decodes a canonical SHA-crypt checksum into its raw digest ordering.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for wrong length, invalid alphabet,
/// noncanonical unused bits, limits, or insufficient capacity.
pub fn decode_sha_crypt_checksum_into(
    algorithm: ShaCryptAlgorithm,
    checksum: &[u8],
    output: &mut [u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    let mut work = WorkBudget::new(limits);
    validate_checksum(algorithm, checksum, limits, &mut work)?;
    let required = algorithm.digest_len();
    require_capacity(required, output.len())?;
    work.charge(checksum.len())?;
    decode_groups(checksum, groups(algorithm), &mut output[..required])?;
    Ok(required)
}

/// Parses and validates one exact SHA-256-crypt or SHA-512-crypt record.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for malformed structure, noncanonical
/// rounds, salt, checksum, or finite-limit failure.
pub fn parse_sha_crypt_record(
    record: &[u8],
    limits: PasswordRecordLimits,
) -> Result<ShaCryptRecord<'_>, PasswordRecordError> {
    require_record(record.len(), limits)?;
    let mut work = WorkBudget::new(limits);
    work.charge(record.len())?;
    let mut fields = record.split(|byte| *byte == b'$');
    if fields.next() != Some(&b""[..]) {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidStructure,
        ));
    }
    let algorithm_field = fields
        .next()
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidStructure))?;
    work.charge(algorithm_field.len())?;
    let algorithm = match algorithm_field {
        b"5" => ShaCryptAlgorithm::Sha256,
        b"6" => ShaCryptAlgorithm::Sha512,
        _ => {
            return Err(PasswordRecordError::new(
                PasswordRecordErrorKind::InvalidPrefix,
            ));
        }
    };
    let first = fields
        .next()
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidStructure))?;
    work.charge(first.len())?;
    let (rounds, salt) = if let Some(decimal) = first.strip_prefix(b"rounds=") {
        work.charge(decimal.len())?;
        let value = parse_decimal(decimal, 1000, 999_999_999)?;
        let salt = fields
            .next()
            .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidStructure))?;
        (ShaCryptRounds::explicit(value)?, salt)
    } else {
        (ShaCryptRounds::implicit(), first)
    };
    let checksum = fields
        .next()
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::InvalidStructure))?;
    if fields.next().is_some() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidStructure,
        ));
    }
    validate_salt(salt, limits, &mut work)?;
    validate_checksum(algorithm, checksum, limits, &mut work)?;
    Ok(ShaCryptRecord {
        algorithm,
        rounds,
        salt,
        checksum,
    })
}

/// Validates SHA-crypt record components and returns the exact generated length.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for invalid rounds, salt, or digest length,
/// finite-limit failure, or arithmetic overflow.
pub fn sha_crypt_record_len(
    algorithm: ShaCryptAlgorithm,
    rounds: ShaCryptRounds,
    salt: &[u8],
    digest: &[u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    let mut work = WorkBudget::new(limits);
    sha_crypt_record_len_with_budget(algorithm, rounds, salt, digest, limits, &mut work)
}

pub(crate) fn sha_crypt_record_len_with_budget(
    algorithm: ShaCryptAlgorithm,
    rounds: ShaCryptRounds,
    salt: &[u8],
    digest: &[u8],
    limits: PasswordRecordLimits,
    work: &mut WorkBudget,
) -> Result<usize, PasswordRecordError> {
    validate_salt(salt, limits, work)?;
    if digest.len() != algorithm.digest_len() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidChecksum,
        ));
    }
    if rounds.value() < 1000 || rounds.value() > 999_999_999 {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidRounds,
        ));
    }
    if digest.len() > limits.max_decoded_output_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    let rounds_len = if rounds.is_explicit() {
        b"rounds=".len() + decimal_len(rounds.value()) + 1
    } else {
        0
    };
    let required = algorithm
        .prefix()
        .len()
        .checked_add(rounds_len)
        .and_then(|length| length.checked_add(salt.len()))
        .and_then(|length| length.checked_add(1))
        .and_then(|length| length.checked_add(algorithm.encoded_checksum_len()))
        .ok_or_else(|| PasswordRecordError::new(PasswordRecordErrorKind::LengthOverflow))?;
    require_generated(required, limits)?;
    Ok(required)
}

/// Generates one canonical SHA-crypt record transactionally.
///
/// `digest` is the already-computed 32-byte or 64-byte SHA-crypt final digest,
/// not a password. This function performs no password hashing.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for invalid salt or digest length,
/// finite-limit failure, arithmetic overflow, or insufficient capacity.
pub fn generate_sha_crypt_record_into(
    algorithm: ShaCryptAlgorithm,
    rounds: ShaCryptRounds,
    salt: &[u8],
    digest: &[u8],
    output: &mut [u8],
    limits: PasswordRecordLimits,
) -> Result<usize, PasswordRecordError> {
    let mut work = WorkBudget::new(limits);
    let required =
        sha_crypt_record_len_with_budget(algorithm, rounds, salt, digest, limits, &mut work)?;
    generate_sha_crypt_record_prevalidated(
        algorithm, rounds, salt, digest, output, required, &mut work,
    )
}

pub(crate) fn generate_sha_crypt_record_prevalidated(
    algorithm: ShaCryptAlgorithm,
    rounds: ShaCryptRounds,
    salt: &[u8],
    digest: &[u8],
    output: &mut [u8],
    required: usize,
    work: &mut WorkBudget,
) -> Result<usize, PasswordRecordError> {
    require_capacity(required, output.len())?;
    work.charge(salt.len())?;
    work.charge(digest.len())?;

    let mut cursor = 0;
    cursor += write_bytes(&mut output[cursor..required], algorithm.prefix());
    if rounds.is_explicit() {
        cursor += write_bytes(&mut output[cursor..required], b"rounds=");
        cursor += write_decimal(rounds.value(), &mut output[cursor..required]);
        output[cursor] = b'$';
        cursor += 1;
    }
    cursor += write_bytes(&mut output[cursor..required], salt);
    output[cursor] = b'$';
    cursor += 1;
    encode_groups(digest, groups(algorithm), &mut output[cursor..required]);
    Ok(required)
}

fn groups(algorithm: ShaCryptAlgorithm) -> &'static [(Option<usize>, Option<usize>, usize, usize)] {
    match algorithm {
        ShaCryptAlgorithm::Sha256 => SHA256_GROUPS,
        ShaCryptAlgorithm::Sha512 => SHA512_GROUPS,
    }
}

fn validate_salt(
    salt: &[u8],
    limits: PasswordRecordLimits,
    work: &mut WorkBudget,
) -> Result<(), PasswordRecordError> {
    require_field(salt.len(), limits)?;
    if salt.len() > 16 {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidSalt,
        ));
    }
    work.charge(salt.len())?;
    if let Some(index) = salt.iter().position(|byte| decode_crypt(*byte).is_none()) {
        return Err(PasswordRecordError::at(
            PasswordRecordErrorKind::InvalidSalt,
            index,
        ));
    }
    Ok(())
}

fn validate_checksum(
    algorithm: ShaCryptAlgorithm,
    checksum: &[u8],
    limits: PasswordRecordLimits,
    work: &mut WorkBudget,
) -> Result<(), PasswordRecordError> {
    require_field(checksum.len(), limits)?;
    if checksum.len() != algorithm.encoded_checksum_len() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::InvalidChecksum,
        ));
    }
    if algorithm.digest_len() > limits.max_decoded_output_bytes() {
        return Err(PasswordRecordError::new(
            PasswordRecordErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    work.charge(checksum.len())?;
    let mut scratch = [0_u8; 64];
    decode_groups(
        checksum,
        groups(algorithm),
        &mut scratch[..algorithm.digest_len()],
    )
}

fn encode_groups(
    digest: &[u8],
    groups: &[(Option<usize>, Option<usize>, usize, usize)],
    output: &mut [u8],
) {
    let mut cursor = 0;
    for &(b2, b1, b0, count) in groups {
        let mut word = (u32::from(b2.map_or(0, |index| digest[index])) << 16)
            | (u32::from(b1.map_or(0, |index| digest[index])) << 8)
            | u32::from(digest[b0]);
        for _ in 0..count {
            output[cursor] = CRYPT_ALPHABET[(word & 0x3f) as usize];
            cursor += 1;
            word >>= 6;
        }
    }
}

fn decode_groups(
    checksum: &[u8],
    groups: &[(Option<usize>, Option<usize>, usize, usize)],
    digest: &mut [u8],
) -> Result<(), PasswordRecordError> {
    let mut cursor = 0;
    for &(b2, b1, b0, count) in groups {
        let mut word = 0_u32;
        for shift in 0..count {
            let value = decode_crypt(checksum[cursor]).ok_or_else(|| {
                PasswordRecordError::at(PasswordRecordErrorKind::InvalidChecksum, cursor)
            })?;
            word |= u32::from(value) << (shift * 6);
            cursor += 1;
        }
        digest[b0] = (word & 0xff) as u8;
        if let Some(index) = b1 {
            digest[index] = ((word >> 8) & 0xff) as u8;
        } else if word >> 8 != 0 {
            return Err(PasswordRecordError::at(
                PasswordRecordErrorKind::InvalidChecksum,
                cursor - 1,
            ));
        }
        if let Some(index) = b2 {
            digest[index] = ((word >> 16) & 0xff) as u8;
        } else if word >> 16 != 0 {
            return Err(PasswordRecordError::at(
                PasswordRecordErrorKind::InvalidChecksum,
                cursor - 1,
            ));
        }
    }
    Ok(())
}

fn decode_crypt(byte: u8) -> Option<u8> {
    CRYPT_ALPHABET
        .iter()
        .position(|candidate| *candidate == byte)
        .and_then(|index| u8::try_from(index).ok())
}
