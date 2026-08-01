//! Token- and allocation-gated assured one-shot operations.

use super::{
    AssuranceError, AssuranceLevel, AssuranceToken, CleanupError, ProtectedMemoryProvider,
    ProtectedSecret, ProtectionError, SecretOperation, Uninitialized, Validated,
};
use crate::v2::{
    Base64, Codec,
    secret::{SecretDecodeError, SecretEncodeError, SecretInput},
    secret_decoder::SecretDecoderState,
    secret_encoder::SecretEncoderState,
    specifications::CodecSettings,
};

/// Redacted assured decode failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AssuredDecodeError {
    /// Token or context evidence is stale.
    Assurance(AssuranceError),
    /// Allocation-specific protection is unavailable or stale.
    Protection(ProtectionError),
    /// Secret decoder rejected the input without localized diagnostics.
    Decode(SecretDecodeError),
    /// Cleanup transferred the consumed allocation to quarantine or tombstone.
    Cleanup(CleanupError),
}

/// Redacted assured encode failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AssuredEncodeError {
    /// Token or context evidence is stale.
    Assurance(AssuranceError),
    /// Allocation-specific protection is unavailable or stale.
    Protection(ProtectionError),
    /// Secret encoder rejected a public bound or internal state.
    Encode(SecretEncodeError),
    /// Cleanup transferred the consumed allocation to quarantine or tombstone.
    Cleanup(CleanupError),
}

impl core::fmt::Display for AssuredDecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Assurance(error) => error.fmt(formatter),
            Self::Protection(error) => error.fmt(formatter),
            Self::Decode(error) => error.fmt(formatter),
            Self::Cleanup(error) => error.fmt(formatter),
        }
    }
}

impl core::fmt::Display for AssuredEncodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Assurance(error) => error.fmt(formatter),
            Self::Protection(error) => error.fmt(formatter),
            Self::Encode(error) => error.fmt(formatter),
            Self::Cleanup(error) => error.fmt(formatter),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AssuredDecodeError {}

#[cfg(feature = "std")]
impl std::error::Error for AssuredEncodeError {}

impl<S: Codec> Base64<S> {
    /// Decodes directly through one protected allocation's typestates.
    ///
    /// The allocation is consumed on every path. Safe code cannot observe its
    /// bytes until the secret result gate succeeds and returns `Validated`.
    pub fn decode_assured<'provider, P, Level>(
        &self,
        token: &AssuranceToken<'provider, Level>,
        allocation: ProtectedSecret<'provider, P, Uninitialized, Level>,
        input: &SecretInput<'_>,
    ) -> Result<ProtectedSecret<'provider, P, Validated, Level>, AssuredDecodeError>
    where
        P: ProtectedMemoryProvider,
        Level: AssuranceLevel,
    {
        token.revalidate().map_err(AssuredDecodeError::Assurance)?;
        let mut allocation = match allocation.begin_unvalidated(token, SecretOperation::Decode) {
            Ok(allocation) => allocation,
            Err((error, allocation)) => {
                return Err(match allocation.try_close() {
                    Ok(_) => AssuredDecodeError::Protection(error),
                    Err(cleanup) => AssuredDecodeError::Cleanup(cleanup),
                });
            }
        };
        let output = match allocation.bytes_mut() {
            Ok(output) => output,
            Err(error) => {
                return Err(match allocation.try_close() {
                    Ok(_) => AssuredDecodeError::Protection(error),
                    Err(cleanup) => AssuredDecodeError::Cleanup(cleanup),
                });
            }
        };
        let result = decode_one_shot(self.settings(), input.classified_bytes(), output);
        let written = match result {
            Ok(written) => written,
            Err(error) => {
                return Err(match allocation.try_close() {
                    Ok(_) => AssuredDecodeError::Decode(error),
                    Err(cleanup) => AssuredDecodeError::Cleanup(cleanup),
                });
            }
        };
        if let Err(error) = allocation.set_initialized_len(written) {
            return Err(match allocation.try_close() {
                Ok(_) => AssuredDecodeError::Protection(error),
                Err(cleanup) => AssuredDecodeError::Cleanup(cleanup),
            });
        }
        match allocation.validate(token) {
            Ok(validated) => Ok(validated),
            Err((error, allocation)) => Err(match allocation.try_close() {
                Ok(_) => AssuredDecodeError::Protection(error),
                Err(cleanup) => AssuredDecodeError::Cleanup(cleanup),
            }),
        }
    }

    /// Encodes directly through one protected allocation's typestates.
    pub fn encode_assured<'provider, P, Level>(
        &self,
        token: &AssuranceToken<'provider, Level>,
        allocation: ProtectedSecret<'provider, P, Uninitialized, Level>,
        input: &SecretInput<'_>,
    ) -> Result<ProtectedSecret<'provider, P, Validated, Level>, AssuredEncodeError>
    where
        P: ProtectedMemoryProvider,
        Level: AssuranceLevel,
    {
        token.revalidate().map_err(AssuredEncodeError::Assurance)?;
        let mut allocation = match allocation.begin_unvalidated(token, SecretOperation::Encode) {
            Ok(allocation) => allocation,
            Err((error, allocation)) => {
                return Err(match allocation.try_close() {
                    Ok(_) => AssuredEncodeError::Protection(error),
                    Err(cleanup) => AssuredEncodeError::Cleanup(cleanup),
                });
            }
        };
        let output = match allocation.bytes_mut() {
            Ok(output) => output,
            Err(error) => {
                return Err(match allocation.try_close() {
                    Ok(_) => AssuredEncodeError::Protection(error),
                    Err(cleanup) => AssuredEncodeError::Cleanup(cleanup),
                });
            }
        };
        let result = encode_one_shot(self.settings(), input.classified_bytes(), output);
        let written = match result {
            Ok(written) => written,
            Err(error) => {
                return Err(match allocation.try_close() {
                    Ok(_) => AssuredEncodeError::Encode(error),
                    Err(cleanup) => AssuredEncodeError::Cleanup(cleanup),
                });
            }
        };
        if let Err(error) = allocation.set_initialized_len(written) {
            return Err(match allocation.try_close() {
                Ok(_) => AssuredEncodeError::Protection(error),
                Err(cleanup) => AssuredEncodeError::Cleanup(cleanup),
            });
        }
        match allocation.validate(token) {
            Ok(validated) => Ok(validated),
            Err((error, allocation)) => Err(match allocation.try_close() {
                Ok(_) => AssuredEncodeError::Protection(error),
                Err(cleanup) => AssuredEncodeError::Cleanup(cleanup),
            }),
        }
    }
}

fn decode_one_shot(
    settings: CodecSettings,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, SecretDecodeError> {
    crate::wipe_bytes(output);
    let mut state = SecretDecoderState::new(settings, output.len())?;
    if let Err(error) = state.update(input, output) {
        crate::wipe_bytes(output);
        return Err(error);
    }
    let candidate = match state.finish() {
        Ok(candidate) => candidate,
        Err(error) => {
            crate::wipe_bytes(output);
            return Err(error);
        }
    };
    let written = candidate.written();
    output[candidate.staged_len..written].copy_from_slice(&candidate.bytes[..candidate.len]);
    crate::wipe_tail(output, written);
    Ok(written)
}

fn encode_one_shot(
    settings: CodecSettings,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, SecretEncodeError> {
    crate::wipe_bytes(output);
    let mut state = SecretEncoderState::new(settings, input.len(), output.len())?;
    if let Err(error) = state.update(input, output) {
        crate::wipe_bytes(output);
        return Err(error);
    }
    match state.finish(output) {
        Ok(written) => Ok(written),
        Err(error) => {
            crate::wipe_bytes(output);
            Err(error)
        }
    }
}
