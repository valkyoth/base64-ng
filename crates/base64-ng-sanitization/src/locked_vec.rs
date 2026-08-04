#![cfg(all(
    feature = "memory-lock",
    any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        target_os = "macos",
        target_os = "ios",
        target_os = "android",
        target_os = "windows",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ),
    not(miri)
))]

use crate::{
    DEFAULT_SECRET_VEC_DECODE_MAX_LEN, LockedDecodeError, preflight::enforce_encoded_input_limit,
};
use base64_ng::{Alphabet, DecodeError, ct::CtEngine};
use sanitization::{LockedSecretVecFillError, ProtectedSecretFillError};

type CheckedLockedVecResult<T> =
    Result<T, LockedDecodeError<LockedSecretVecFillError<DecodeError>>>;

pub(crate) fn validate_before_locked_vec_allocation<A, const PAD: bool, T, F>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
    allocate_and_decode: F,
) -> CheckedLockedVecResult<T>
where
    A: Alphabet,
    F: FnOnce(usize) -> CheckedLockedVecResult<T>,
{
    let required = preflight_locked_vec(engine, input).map_err(LockedDecodeError::Operation)?;
    allocate_and_decode(required)
}

pub(crate) fn preflight_locked_vec<A, const PAD: bool>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
) -> Result<usize, LockedSecretVecFillError<DecodeError>>
where
    A: Alphabet,
{
    enforce_encoded_input_limit::<PAD>(DEFAULT_SECRET_VEC_DECODE_MAX_LEN, input.len())
        .map_err(|_| LockedSecretVecFillError::Fill(DecodeError::InvalidLength))?;
    let required = engine
        .decoded_len(input)
        .map_err(LockedSecretVecFillError::Fill)?;
    if required > DEFAULT_SECRET_VEC_DECODE_MAX_LEN {
        return Err(LockedSecretVecFillError::Length(
            sanitization::LengthError {
                expected: DEFAULT_SECRET_VEC_DECODE_MAX_LEN,
                actual: required,
            },
        ));
    }
    Ok(required)
}

pub(crate) fn map_protected_vec_error(
    error: ProtectedSecretFillError<DecodeError>,
) -> LockedDecodeError<LockedSecretVecFillError<DecodeError>> {
    match error {
        ProtectedSecretFillError::CapacityLimit { maximum, actual } => {
            LockedDecodeError::Operation(LockedSecretVecFillError::Length(
                sanitization::LengthError {
                    expected: maximum,
                    actual,
                },
            ))
        }
        ProtectedSecretFillError::Protection(_) | ProtectedSecretFillError::Integrity(_) => {
            LockedDecodeError::DegradedProtection
        }
        ProtectedSecretFillError::Fill(error) => {
            LockedDecodeError::Operation(LockedSecretVecFillError::Fill(error))
        }
        ProtectedSecretFillError::Length(error) => {
            LockedDecodeError::Operation(LockedSecretVecFillError::Length(error))
        }
    }
}
