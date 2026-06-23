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
//! ordinary strict decoder. `sanitization` 1.2.1's native [`ct`] primitives are
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
use base64_ng::{Alphabet, DecodeError, ct::CtEngine};
use sanitization::{
    SecretBytes, SecureSanitize,
    ct::{Choice, ConstantTimeEq},
};

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
use sanitization::{LockedSecretBytes, LockedSecretBytesGenerateError};

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

/// Native `sanitization::ct` comparison helpers for decoded secret containers.
///
/// Length is public: mismatched lengths return [`Choice::FALSE`] immediately.
/// Use fixed-size protocol tokens when length must not vary. Converting
/// [`Choice`] to `bool` is declassification and requires an explicit reason.
pub trait SanitizationCtEqExt {
    /// Compare this secret container with `expected` using
    /// `sanitization`'s native constant-time-oriented equality primitive.
    #[must_use = "compose Choice values or declassify explicitly with a reason"]
    fn sanitization_ct_eq(&self, expected: &[u8]) -> Choice;

    /// Convenience boolean wrapper around [`Self::sanitization_ct_eq`].
    ///
    /// `reason` is passed through to [`Choice::declassify`] so reviews can
    /// audit every branch point where a secret-derived decision becomes public.
    #[must_use]
    fn sanitization_verify(&self, expected: &[u8], reason: &'static str) -> bool {
        self.sanitization_ct_eq(expected).declassify(reason)
    }
}

impl<const N: usize> SanitizationCtEqExt for SecretBytes<N> {
    fn sanitization_ct_eq(&self, expected: &[u8]) -> Choice {
        <SecretBytes<N> as ConstantTimeEq<[u8]>>::ct_eq(self, expected)
    }
}

#[cfg(feature = "alloc")]
impl SanitizationCtEqExt for SecretVec {
    fn sanitization_ct_eq(&self, expected: &[u8]) -> Choice {
        <SecretVec as ConstantTimeEq<[u8]>>::ct_eq(self, expected)
    }
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
impl<const N: usize> SanitizationCtEqExt for LockedSecretBytes<N> {
    fn sanitization_ct_eq(&self, expected: &[u8]) -> Choice {
        <LockedSecretBytes<N> as ConstantTimeEq<[u8]>>::ct_eq(self, expected)
    }
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
impl SanitizationCtEqExt for LockedSecretVec {
    fn sanitization_ct_eq(&self, expected: &[u8]) -> Choice {
        <LockedSecretVec as ConstantTimeEq<[u8]>>::ct_eq(self, expected)
    }
}

/// Compare two byte slices through `sanitization::ct` with public length.
///
/// This is useful when callers want the same native [`Choice`] type without
/// first wrapping bytes in a `sanitization` secret container.
#[must_use = "compose Choice values or declassify explicitly with a reason"]
pub fn sanitization_ct_eq_public_len(left: &[u8], right: &[u8]) -> Choice {
    ct::eq_public_len(left, right)
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

    /// Decode `input` directly into fixed-size locked secret storage.
    ///
    /// Enable the `memory-lock` feature, or `high-assurance` for
    /// `memory-lock` plus `canary-check` and `random-canary`, to use this
    /// helper on supported native targets. Decode uses private stack staging,
    /// then copies into locked storage only after the full decode succeeds.
    ///
    /// The decoded length must exactly match `N`. A length mismatch returns
    /// [`SanitizationDecodeError::LengthMismatch`] inside the
    /// `sanitization` generation error.
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
    ) -> Result<LockedSecretBytes<N>, LockedSecretBytesGenerateError<SanitizationDecodeError>>;

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
    /// Enable the `memory-lock` feature, or `high-assurance` for
    /// `memory-lock` plus `canary-check` and `random-canary`, to use this
    /// helper on supported native targets. The locked mapping is created at the
    /// exact decoded capacity and bytes are written directly into that mapping
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

        debug_assert_eq!(written, N);
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
    ) -> Result<LockedSecretBytes<N>, LockedSecretBytesGenerateError<SanitizationDecodeError>> {
        let required = self
            .decoded_len(input)
            .map_err(|error| LockedSecretBytesGenerateError::Generate(error.into()))?;
        if required != N {
            return Err(LockedSecretBytesGenerateError::Generate(
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
                return Err(LockedSecretBytesGenerateError::Generate(error.into()));
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
