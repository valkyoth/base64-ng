//! Protected two-allocation decode integration for the 2.0 codec.

use base64_ng::{
    Base64, Codec,
    secret::{SecretDecodeError, SecretFrame, SecretInput},
};
use sanitization::{
    LockedSecretBytes, LockedSecretInitializeError, LockedSecretVec, ProtectedSecretFillError,
    SecureSanitize,
};

use crate::locked;

/// Which protected allocation failed during a two-allocation decode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedAllocation {
    /// Private fixed-work decode staging.
    Staging,
    /// Final destination returned only after the result gate.
    Destination,
}

/// Redacted failure from a 2.0 protected sanitization decode.
///
/// The error preserves the failed allocation and failure class without
/// retaining input bytes, plaintext, secret addresses, or localized invalid
/// input details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SanitizationProtectedDecodeError {
    /// Required controls could not be established before decoder work.
    Protection {
        /// Allocation whose protection setup failed.
        allocation: ProtectedAllocation,
    },
    /// Canary validation failed for one protected allocation.
    Integrity {
        /// Allocation whose integrity check failed.
        allocation: ProtectedAllocation,
    },
    /// The fixed-work secret decoder rejected the frame.
    Decode(SecretDecodeError),
    /// A valid frame did not produce the required fixed-size output.
    LengthMismatch {
        /// Required public output length.
        expected: usize,
        /// Actual public output length.
        actual: usize,
    },
    /// An upstream protected constructor reported an inconsistent length.
    ProtectedLength {
        /// Allocation whose protected length was inconsistent.
        allocation: ProtectedAllocation,
    },
    /// A protected destination could not be transferred after initialization.
    StateTransition,
}

impl core::fmt::Display for SanitizationProtectedDecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Protection { allocation } => {
                write!(
                    formatter,
                    "required {allocation:?} protection is unavailable"
                )
            }
            Self::Integrity { allocation } => {
                write!(
                    formatter,
                    "{allocation:?} protected-storage integrity failure"
                )
            }
            Self::Decode(error) => error.fmt(formatter),
            Self::LengthMismatch { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} decoded bytes, produced {actual}"
                )
            }
            Self::ProtectedLength { allocation } => {
                write!(formatter, "{allocation:?} protected-storage length failure")
            }
            Self::StateTransition => formatter.write_str("protected destination transition failed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SanitizationProtectedDecodeError {}

/// 2.0 decode helpers with separately protected staging and destination.
///
/// Both mappings are established under the companion's required protection
/// request before [`SecretFrame`] receives the encoded input. The input,
/// staging, and destination are separate logical ranges. Invalid input wipes
/// both mappings before the error is returned.
///
/// For provider-owned quarantine, generation accounting, and single-allocation
/// no-copy finalization, use [`Base64::decode_assured`] with
/// `base64_ng::assurance::ProtectedSecret` instead. A `sanitization` mapping
/// cannot be claimed as a core assurance allocation because its raw ownership
/// and teardown state are intentionally not exported.
pub trait SanitizationProtectedDecodeExt {
    /// Decode exactly `N` bytes through protected staging into protected final
    /// storage.
    ///
    /// # Errors
    ///
    /// Returns a redacted protection, integrity, decode, or exact-length
    /// failure. Protection failures occur before decoder work.
    fn decode_sanitization_protected_bytes<const N: usize>(
        &self,
        input: &SecretInput<'_>,
    ) -> Result<LockedSecretBytes<N>, SanitizationProtectedDecodeError>;

    /// Decode into a protected dynamic destination with public capacity `MAX`.
    ///
    /// Both mappings allocate `MAX` bytes before decoder work. This makes the
    /// Base64-owned allocation shape independent of frame validity while
    /// bounding memory use to two protected mappings of `MAX` bytes.
    ///
    /// # Errors
    ///
    /// Returns a redacted protection, integrity, decode, or protected-length
    /// failure.
    fn decode_sanitization_protected_vec<const MAX: usize>(
        &self,
        input: &SecretInput<'_>,
    ) -> Result<LockedSecretVec, SanitizationProtectedDecodeError>;
}

impl<S: Codec> SanitizationProtectedDecodeExt for Base64<S> {
    fn decode_sanitization_protected_bytes<const N: usize>(
        &self,
        input: &SecretInput<'_>,
    ) -> Result<LockedSecretBytes<N>, SanitizationProtectedDecodeError> {
        let request = locked::required_secret_protection();
        let staging = LockedSecretBytes::<N>::zeroed_with_protection(request).map_err(|_| {
            SanitizationProtectedDecodeError::Protection {
                allocation: ProtectedAllocation::Staging,
            }
        })?;
        let mut destination = None;
        let staging = staging
            .try_init_with(|staging_bytes| {
                let target =
                    LockedSecretBytes::<N>::zeroed_with_protection(request).map_err(|_| {
                        SanitizationProtectedDecodeError::Protection {
                            allocation: ProtectedAllocation::Destination,
                        }
                    })?;
                let target = target
                    .try_init_with(|output| {
                        let written = decode_frame(self, input, staging_bytes, output)?;
                        if written != N {
                            return Err(SanitizationProtectedDecodeError::LengthMismatch {
                                expected: N,
                                actual: written,
                            });
                        }
                        Ok(())
                    })
                    .map_err(map_destination_initialize_error)?;
                staging_bytes.secure_sanitize();
                destination = Some(target);
                Ok(())
            })
            .map_err(map_staging_initialize_error)?;
        drop(staging);
        destination.ok_or(SanitizationProtectedDecodeError::StateTransition)
    }

    fn decode_sanitization_protected_vec<const MAX: usize>(
        &self,
        input: &SecretInput<'_>,
    ) -> Result<LockedSecretVec, SanitizationProtectedDecodeError> {
        let request = locked::required_secret_protection();
        let mut staging =
            LockedSecretVec::try_from_exact_len_with_protection(MAX, request, |bytes| {
                bytes.secure_sanitize();
                Ok::<(), core::convert::Infallible>(())
            })
            .map_err(map_staging_fill_error)?;

        LockedSecretVec::try_from_capacity_bounded_with_protection(MAX, MAX, request, |output| {
            staging
                .try_with_secret_mut(|staging_bytes| {
                    decode_frame(self, input, staging_bytes, output)
                })
                .map_err(|_| SanitizationProtectedDecodeError::Integrity {
                    allocation: ProtectedAllocation::Staging,
                })?
        })
        .map_err(map_destination_fill_error)
    }
}

fn decode_frame<S: Codec>(
    codec: &Base64<S>,
    input: &SecretInput<'_>,
    staging: &mut [u8],
    output: &mut [u8],
) -> Result<usize, SanitizationProtectedDecodeError> {
    let mut frame = SecretFrame::new(codec, output.len(), staging, output)
        .map_err(SanitizationProtectedDecodeError::Decode)?;
    frame
        .update(input)
        .map_err(SanitizationProtectedDecodeError::Decode)?;
    let guarded = frame
        .finish()
        .map_err(SanitizationProtectedDecodeError::Decode)?;
    let written = guarded.len();
    drop(guarded.declassify());
    Ok(written)
}

fn map_staging_initialize_error(
    error: LockedSecretInitializeError<SanitizationProtectedDecodeError>,
) -> SanitizationProtectedDecodeError {
    match error {
        LockedSecretInitializeError::Integrity(_) => SanitizationProtectedDecodeError::Integrity {
            allocation: ProtectedAllocation::Staging,
        },
        LockedSecretInitializeError::Generate(error) => error,
    }
}

fn map_destination_initialize_error(
    error: LockedSecretInitializeError<SanitizationProtectedDecodeError>,
) -> SanitizationProtectedDecodeError {
    match error {
        LockedSecretInitializeError::Integrity(_) => SanitizationProtectedDecodeError::Integrity {
            allocation: ProtectedAllocation::Destination,
        },
        LockedSecretInitializeError::Generate(error) => error,
    }
}

fn map_staging_fill_error(
    error: ProtectedSecretFillError<core::convert::Infallible>,
) -> SanitizationProtectedDecodeError {
    map_fill_error(error, ProtectedAllocation::Staging)
}

fn map_destination_fill_error(
    error: ProtectedSecretFillError<SanitizationProtectedDecodeError>,
) -> SanitizationProtectedDecodeError {
    map_fill_error(error, ProtectedAllocation::Destination)
}

fn map_fill_error<E>(
    error: ProtectedSecretFillError<E>,
    allocation: ProtectedAllocation,
) -> SanitizationProtectedDecodeError
where
    E: IntoSanitizationProtectedDecodeError,
{
    match error {
        ProtectedSecretFillError::CapacityLimit { .. } | ProtectedSecretFillError::Length(_) => {
            SanitizationProtectedDecodeError::ProtectedLength { allocation }
        }
        ProtectedSecretFillError::Protection(_) => {
            SanitizationProtectedDecodeError::Protection { allocation }
        }
        ProtectedSecretFillError::Fill(error) => error.into_decode_error(),
        ProtectedSecretFillError::Integrity(_) => {
            SanitizationProtectedDecodeError::Integrity { allocation }
        }
    }
}

trait IntoSanitizationProtectedDecodeError {
    fn into_decode_error(self) -> SanitizationProtectedDecodeError;
}

impl IntoSanitizationProtectedDecodeError for SanitizationProtectedDecodeError {
    fn into_decode_error(self) -> SanitizationProtectedDecodeError {
        self
    }
}

impl IntoSanitizationProtectedDecodeError for core::convert::Infallible {
    fn into_decode_error(self) -> SanitizationProtectedDecodeError {
        match self {}
    }
}
