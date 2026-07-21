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
//! ordinary strict decoder. `sanitization` 2.0.1's native [`ct`] primitives are
//! re-exported for callers that want `Choice`-based verification after decode.
//! Enable `memory-lock` or `high-assurance` for locked secret containers on
//! supported native targets.
//!
//! ```
//! use base64_ng::ct;
//! use base64_ng_sanitization::{CtDecodeSanitizationExt, SanitizationCtEqExt};
//!
//! let secret = ct::STANDARD
//!     .decode_secret_bytes::<5>(b"aGVsbG8=")
//!     .unwrap();
//! assert!(secret.sanitization_verify(b"hello", "example declassifies equality"));
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

mod compare;
mod error;

use base64_ng::{Alphabet, ct::CtEngine};
pub use compare::{LockedSanitizationCtEqExt, SanitizationCtEqExt, sanitization_ct_eq_public_len};
pub use error::SanitizationDecodeError;
use sanitization::{SecretBytes, SecureSanitize};

#[cfg(any(feature = "alloc", feature = "memory-lock"))]
use base64_ng::DecodeError;

#[cfg(feature = "alloc")]
use sanitization::SecretVec;

#[cfg(all(
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
        all(target_arch = "wasm32", feature = "wasm-compat"),
    )
))]
use sanitization::{LockedSecretBytes, LockedSecretBytesFillError};

#[cfg(all(
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
use sanitization::{LockedSecretVec, LockedSecretVecFillError};

pub use sanitization::ct;

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

    /// Decode `input` directly into fixed-size locked secret storage.
    ///
    /// Enable the `memory-lock` feature, or `high-assurance` for
    /// the hardened native controls used by `high-assurance`, to use this
    /// helper on supported native targets. Decode uses private stack staging,
    /// then copies into locked storage only after the full decode succeeds.
    ///
    /// The decoded length must exactly match `N`. A length mismatch returns
    /// [`SanitizationDecodeError::LengthMismatch`] inside the
    /// `sanitization` fill error.
    ///
    /// # Errors
    ///
    /// Returns a `sanitization` memory error if locked storage cannot be
    /// created. Returns [`SanitizationDecodeError::Decode`] if Base64 decoding
    /// fails, or [`SanitizationDecodeError::LengthMismatch`] if the decoded
    /// length does not exactly equal `N`.
    #[cfg(all(
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
            all(target_arch = "wasm32", feature = "wasm-compat"),
        )
    ))]
    fn decode_locked_secret_bytes<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretBytes<N>, LockedSecretBytesFillError<SanitizationDecodeError>>;

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

    /// Decode `input` directly into heap-backed locked secret storage.
    ///
    /// Enable the `memory-lock` feature, or `high-assurance` for the hardened
    /// native controls used by `high-assurance`, to use this helper on
    /// supported native targets. The locked mapping is created at the exact
    /// decoded capacity and bytes are written directly into that mapping
    /// through `sanitization::LockedSecretVec::try_from_capacity`.
    ///
    /// # Errors
    ///
    /// Returns a `sanitization` memory error if locked storage cannot be
    /// created. Returns [`DecodeError`] from the fill branch if Base64 decoding
    /// fails.
    #[cfg(all(
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
    fn decode_locked_secret_vec(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretVec, LockedSecretVecFillError<DecodeError>>;
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

        if written != N {
            output.secure_sanitize();
            staging.secure_sanitize();
            return Err(SanitizationDecodeError::LengthMismatch {
                expected: N,
                actual: written,
            });
        }

        let secret = SecretBytes::from_fn(|index| output[index]);
        output.secure_sanitize();
        staging.secure_sanitize();
        Ok(secret)
    }

    #[cfg(all(
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
            all(target_arch = "wasm32", feature = "wasm-compat"),
        )
    ))]
    fn decode_locked_secret_bytes<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretBytes<N>, LockedSecretBytesFillError<SanitizationDecodeError>> {
        let required = self
            .decoded_len(input)
            .map_err(|error| LockedSecretBytesFillError::Generate(error.into()))?;
        if required != N {
            return Err(LockedSecretBytesFillError::Generate(
                SanitizationDecodeError::LengthMismatch {
                    expected: N,
                    actual: required,
                },
            ));
        }

        let mut output = [0u8; N];
        let mut staging = [0u8; N];
        let written = match self.decode_slice_staged_clear_tail(input, &mut output, &mut staging) {
            Ok(written) => written,
            Err(error) => {
                output.secure_sanitize();
                staging.secure_sanitize();
                return Err(LockedSecretBytesFillError::Generate(error.into()));
            }
        };
        staging.secure_sanitize();

        let result = LockedSecretBytes::try_from_fill(|locked| {
            if written != N {
                return Err(SanitizationDecodeError::LengthMismatch {
                    expected: N,
                    actual: written,
                });
            }

            locked.copy_from_slice(&output);
            Ok(())
        });
        output.secure_sanitize();
        result
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

    #[cfg(all(
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
    fn decode_locked_secret_vec(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretVec, LockedSecretVecFillError<DecodeError>> {
        let required = self
            .decoded_len(input)
            .map_err(LockedSecretVecFillError::Fill)?;
        LockedSecretVec::try_from_capacity(required, |output| {
            self.decode_slice_clear_tail(input, output)
        })
    }
}

#[cfg(test)]
mod tests;
