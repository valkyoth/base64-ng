use alloc::{string::String, vec::Vec};

use crate::{
    PasslibPbkdf2Algorithm, PasswordRecordError, PasswordRecordErrorKind, PasswordRecordLimits,
    ShaCryptAlgorithm, ShaCryptRounds,
    pbkdf2::{generate_pbkdf2_record_prevalidated, preflight_pbkdf2_generation},
    sha_crypt::{generate_sha_crypt_record_prevalidated, preflight_sha_crypt_generation},
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
    generate_pbkdf2_record_with_reserver(algorithm, rounds, salt, checksum, limits, reserve_exact)
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
    generate_sha_crypt_record_with_reserver(algorithm, rounds, salt, digest, limits, reserve_exact)
}

fn generate_pbkdf2_record_with_reserver<R>(
    algorithm: PasslibPbkdf2Algorithm,
    rounds: u32,
    salt: &[u8],
    checksum: &[u8],
    limits: PasswordRecordLimits,
    reserve: R,
) -> Result<String, PasswordRecordError>
where
    R: FnOnce(&mut Vec<u8>, usize) -> Result<(), PasswordRecordError>,
{
    let (required, salt_len) =
        preflight_pbkdf2_generation(algorithm, rounds, salt, checksum, limits)?;
    generate_string(required, reserve, |output| {
        generate_pbkdf2_record_prevalidated(
            algorithm, rounds, salt, checksum, output, required, salt_len,
        )
    })
}

fn generate_sha_crypt_record_with_reserver<R>(
    algorithm: ShaCryptAlgorithm,
    rounds: ShaCryptRounds,
    salt: &[u8],
    digest: &[u8],
    limits: PasswordRecordLimits,
    reserve: R,
) -> Result<String, PasswordRecordError>
where
    R: FnOnce(&mut Vec<u8>, usize) -> Result<(), PasswordRecordError>,
{
    let required = preflight_sha_crypt_generation(algorithm, rounds, salt, digest, limits)?;
    generate_string(required, reserve, |output| {
        generate_sha_crypt_record_prevalidated(algorithm, rounds, salt, digest, output, required)
    })
}

fn reserve_exact(bytes: &mut Vec<u8>, required: usize) -> Result<(), PasswordRecordError> {
    bytes
        .try_reserve_exact(required)
        .map_err(|_| PasswordRecordError::new(PasswordRecordErrorKind::AllocationFailed))
}

fn generate_string<R, G>(
    required: usize,
    reserve: R,
    generate: G,
) -> Result<String, PasswordRecordError>
where
    R: FnOnce(&mut Vec<u8>, usize) -> Result<(), PasswordRecordError>,
    G: FnOnce(&mut [u8]) -> usize,
{
    let mut bytes = Vec::new();
    reserve(&mut bytes, required)?;
    bytes.resize(required, 0);
    let written = generate(&mut bytes);
    bytes.truncate(written);
    String::from_utf8(bytes)
        .map_err(|_| PasswordRecordError::new(PasswordRecordErrorKind::BackendFailure))
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use super::*;

    fn runtime_fixture_bytes<const N: usize>(domain: u8) -> [u8; N] {
        let seed = core::hint::black_box(domain);
        core::array::from_fn(|index| {
            b'a' + seed.wrapping_add(u8::try_from(index % 251).unwrap()) % 26
        })
    }

    #[test]
    fn rejected_work_budget_never_reaches_reservation() {
        let pbkdf2_salt = runtime_fixture_bytes::<4>(0x19);
        let pbkdf2_checksum = runtime_fixture_bytes::<32>(0x2b);
        let pbkdf2_reserve_called = Cell::new(false);
        let pbkdf2_limits = PasswordRecordLimits::new(256, 128, 64, 64, 256, 35);
        let error = generate_pbkdf2_record_with_reserver(
            PasslibPbkdf2Algorithm::Sha256,
            29_000,
            &pbkdf2_salt,
            &pbkdf2_checksum,
            pbkdf2_limits,
            |_, _| {
                pbkdf2_reserve_called.set(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), PasswordRecordErrorKind::WorkLimitExceeded);
        assert!(!pbkdf2_reserve_called.get());

        let sha_salt = runtime_fixture_bytes::<4>(0x3d);
        let sha_digest = runtime_fixture_bytes::<32>(0x4f);
        let sha_reserve_called = Cell::new(false);
        let sha_limits = PasswordRecordLimits::new(256, 128, 64, 64, 256, 39);
        let error = generate_sha_crypt_record_with_reserver(
            ShaCryptAlgorithm::Sha256,
            ShaCryptRounds::implicit(),
            &sha_salt,
            &sha_digest,
            sha_limits,
            |_, _| {
                sha_reserve_called.set(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), PasswordRecordErrorKind::WorkLimitExceeded);
        assert!(!sha_reserve_called.get());
    }
}
