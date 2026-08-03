use alloc::{string::String, vec::Vec};

use crate::{
    PasslibPbkdf2Algorithm, PasswordRecordError, PasswordRecordErrorKind, PasswordRecordLimits,
    ShaCryptAlgorithm, ShaCryptRounds, generate_pbkdf2_record_into, generate_sha_crypt_record_into,
    pbkdf2_record_len, sha_crypt_record_len,
};

/// Generates one canonical Passlib PBKDF2 record into an exact allocation.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for validation, limit, reservation, or
/// internal ASCII-invariant failure.
pub fn generate_pbkdf2_record(
    algorithm: PasslibPbkdf2Algorithm,
    rounds: u32,
    salt: &[u8],
    checksum: &[u8],
    limits: PasswordRecordLimits,
) -> Result<String, PasswordRecordError> {
    let required = pbkdf2_record_len(algorithm, rounds, salt, checksum, limits)?;
    generate_string(required, |output| {
        generate_pbkdf2_record_into(algorithm, rounds, salt, checksum, output, limits)
    })
}

/// Generates one canonical SHA-crypt record into an exact allocation.
///
/// # Errors
///
/// Returns [`PasswordRecordError`] for validation, limit, reservation, or
/// internal ASCII-invariant failure.
pub fn generate_sha_crypt_record(
    algorithm: ShaCryptAlgorithm,
    rounds: ShaCryptRounds,
    salt: &[u8],
    digest: &[u8],
    limits: PasswordRecordLimits,
) -> Result<String, PasswordRecordError> {
    let required = sha_crypt_record_len(algorithm, rounds, salt, digest, limits)?;
    generate_string(required, |output| {
        generate_sha_crypt_record_into(algorithm, rounds, salt, digest, output, limits)
    })
}

fn generate_string(
    required: usize,
    generate: impl FnOnce(&mut [u8]) -> Result<usize, PasswordRecordError>,
) -> Result<String, PasswordRecordError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(required)
        .map_err(|_| PasswordRecordError::new(PasswordRecordErrorKind::AllocationFailed))?;
    bytes.resize(required, 0);
    let written = generate(&mut bytes)?;
    bytes.truncate(written);
    String::from_utf8(bytes)
        .map_err(|_| PasswordRecordError::new(PasswordRecordErrorKind::BackendFailure))
}
