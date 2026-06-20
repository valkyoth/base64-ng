#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional `sanitization` integration helpers for `base64-ng`.
//!
//! This crate deliberately lives outside the core `base64-ng` package so the
//! base crate keeps its zero-runtime-dependency contract. Applications that
//! already admit `sanitization` can opt into this companion crate and decode
//! secret-bearing Base64 directly into `sanitization` secret containers.
//!
//! The extension trait targets [`base64_ng::ct::CtEngine`] rather than the
//! ordinary strict decoder. That keeps secret-container ergonomics aligned with
//! the constant-time-oriented decode path.
//!
//! ```
//! use base64_ng::ct;
//! use base64_ng_sanitization::CtDecodeSanitizationExt;
//!
//! let secret = ct::STANDARD
//!     .decode_secret_bytes::<5>(b"aGVsbG8=")
//!     .unwrap();
//!
//! secret.expose_secret(|bytes| assert_eq!(bytes, b"hello"));
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

use base64_ng::{Alphabet, DecodeError, ct::CtEngine};
use sanitization::{SecretBytes, SecureSanitize};

#[cfg(feature = "alloc")]
use sanitization::SecretVec;

/// Error returned by fixed-size sanitization decode helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SanitizationDecodeError {
    /// The Base64 decoder rejected the input.
    Decode(DecodeError),
    /// The decoded byte length does not match the requested fixed-size secret.
    LengthMismatch {
        /// Expected decoded byte length.
        expected: usize,
        /// Actual decoded byte length.
        actual: usize,
    },
}

impl core::fmt::Display for SanitizationDecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(formatter),
            Self::LengthMismatch { expected, actual } => write!(
                formatter,
                "decoded Base64 length mismatch: expected {expected}, actual {actual}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SanitizationDecodeError {}

impl From<DecodeError> for SanitizationDecodeError {
    #[inline]
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}

/// Extension helpers for decoding with [`base64_ng::ct::CtEngine`] into
/// `sanitization` secret containers.
///
/// These helpers use `decode_slice_staged_clear_tail` for fixed-size
/// [`SecretBytes`] output so the public final container is only populated after
/// the constant-time-oriented decode succeeds.
pub trait CtDecodeSanitizationExt {
    /// Decode `input` into a fixed-size clear-on-drop secret.
    ///
    /// The decoded length must exactly match `N`. A length mismatch returns
    /// [`SanitizationDecodeError::LengthMismatch`] and no secret container is
    /// returned.
    ///
    /// This method uses a private stack staging buffer and clears temporary
    /// buffers before returning.
    ///
    /// # Errors
    ///
    /// Returns [`SanitizationDecodeError::Decode`] if Base64 decoding fails.
    /// Returns [`SanitizationDecodeError::LengthMismatch`] if the decoded
    /// length does not exactly equal `N`.
    fn decode_secret_bytes<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretBytes<N>, SanitizationDecodeError>;

    /// Decode `input` into a heap-backed clear-on-drop secret vector.
    ///
    /// Enable the `alloc` feature to use this helper. For shared-memory or
    /// enclave-adjacent deployments where the final heap allocation must not
    /// contain transient plaintext from rejected input, prefer
    /// [`Self::decode_secret_vec_staged`] with a stack staging capacity large
    /// enough for the decoded value.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if Base64 decoding fails or the decoded length
    /// cannot be represented for the selected padding policy.
    #[cfg(feature = "alloc")]
    fn decode_secret_vec(&self, input: &[u8]) -> Result<SecretVec, DecodeError>;

    /// Decode `input` through a private stack staging buffer into a
    /// heap-backed clear-on-drop secret vector.
    ///
    /// `STAGE` must be at least the decoded byte length of `input`.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if Base64 decoding fails or `STAGE` is smaller
    /// than the required decoded length.
    #[cfg(feature = "alloc")]
    fn decode_secret_vec_staged<const STAGE: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretVec, DecodeError>;
}

impl<A, const PAD: bool> CtDecodeSanitizationExt for CtEngine<A, PAD>
where
    A: Alphabet,
{
    fn decode_secret_bytes<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretBytes<N>, SanitizationDecodeError> {
        let required = self.decoded_len(input)?;
        if required != N {
            return Err(SanitizationDecodeError::LengthMismatch {
                expected: N,
                actual: required,
            });
        }

        let mut output = [0u8; N];
        let mut staging = [0u8; N];
        let written = match self.decode_slice_staged_clear_tail(input, &mut output, &mut staging) {
            Ok(written) => written,
            Err(error) => {
                output.secure_sanitize();
                staging.secure_sanitize();
                return Err(error.into());
            }
        };

        debug_assert_eq!(written, N);
        let secret = SecretBytes::from_fn(|index| output[index]);
        output.secure_sanitize();
        staging.secure_sanitize();
        Ok(secret)
    }

    #[cfg(feature = "alloc")]
    fn decode_secret_vec(&self, input: &[u8]) -> Result<SecretVec, DecodeError> {
        let required = self.decoded_len(input)?;
        let mut output = alloc::vec![0; required];
        let written = self.decode_slice_clear_tail(input, &mut output)?;
        output.truncate(written);
        Ok(SecretVec::from_vec(output))
    }

    #[cfg(feature = "alloc")]
    fn decode_secret_vec_staged<const STAGE: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretVec, DecodeError> {
        let required = self.decoded_len(input)?;
        let mut output = alloc::vec![0; required];
        let mut staging = [0u8; STAGE];
        let written = match self.decode_slice_staged_clear_tail(input, &mut output, &mut staging) {
            Ok(written) => written,
            Err(error) => {
                output.secure_sanitize();
                staging.secure_sanitize();
                return Err(error);
            }
        };
        output.truncate(written);
        staging.secure_sanitize();
        Ok(SecretVec::from_vec(output))
    }
}

#[cfg(test)]
mod tests {
    use super::{CtDecodeSanitizationExt, SanitizationDecodeError};
    use base64_ng::{DecodeError, ct};

    #[test]
    fn decodes_fixed_secret_bytes() {
        let secret = ct::STANDARD.decode_secret_bytes::<5>(b"aGVsbG8=").unwrap();
        secret.expose_secret(|bytes| assert_eq!(bytes, b"hello"));
    }

    #[test]
    fn fixed_secret_bytes_reject_length_mismatch() {
        assert_eq!(
            ct::STANDARD
                .decode_secret_bytes::<4>(b"aGVsbG8=")
                .unwrap_err(),
            SanitizationDecodeError::LengthMismatch {
                expected: 4,
                actual: 5
            }
        );
    }

    #[test]
    fn fixed_secret_bytes_reports_decode_error() {
        assert_eq!(
            ct::STANDARD
                .decode_secret_bytes::<5>(b"aGVsbG8!")
                .unwrap_err(),
            SanitizationDecodeError::Decode(DecodeError::InvalidInput)
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn decodes_secret_vec() {
        let secret = ct::STANDARD.decode_secret_vec(b"aGVsbG8=").unwrap();
        secret.with_secret(|bytes| assert_eq!(bytes, b"hello"));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn decodes_secret_vec_staged() {
        let secret = ct::STANDARD
            .decode_secret_vec_staged::<5>(b"aGVsbG8=")
            .unwrap();
        secret.with_secret(|bytes| assert_eq!(bytes, b"hello"));
    }
}
