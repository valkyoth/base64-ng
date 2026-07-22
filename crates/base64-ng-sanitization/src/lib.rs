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
//! ordinary strict decoder. `sanitization` 2.0.3's native [`ct`] primitives are
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
#[cfg(test)]
mod compare_tests;
mod decode_impl;
mod error;
#[cfg(feature = "memory-lock")]
mod locked;
#[cfg(test)]
mod locked_tests;
#[cfg(feature = "memory-lock")]
mod locked_vec;
#[cfg(feature = "memory-lock")]
mod protected_decode;
#[cfg(test)]
mod protected_tests;

pub use compare::{LockedSanitizationCtEqExt, SanitizationCtEqExt, sanitization_ct_eq_public_len};
pub use error::{LockedDecodeError, SanitizationDecodeError};
#[cfg(feature = "memory-lock")]
pub use protected_decode::CtDecodeSanitizationProtectedExt;
use sanitization::SecretBytes;

#[cfg(any(feature = "alloc", all(feature = "memory-lock", not(miri))))]
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
use sanitization::{LockedSecretBytes, LockedSecretBytesFillError, LockedSecretBytesGenerateError};

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
    /// `sanitization` generation error.
    ///
    /// # Errors
    ///
    /// Returns a `sanitization` memory error if locked storage cannot be
    /// created. Returns [`SanitizationDecodeError::Decode`] if Base64 decoding
    /// fails, or [`SanitizationDecodeError::LengthMismatch`] if the decoded
    /// length does not exactly equal `N`.
    ///
    /// # Security
    ///
    /// Successful construction does not prove that preferred dump and fork
    /// controls were established. Inspect `protection_report()` or use
    /// [`Self::decode_locked_secret_bytes_checked`].
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

    /// Decode through stack staging and initialize fixed-size locked storage
    /// through sanitization's integrity-checked fill API.
    ///
    /// This additive method exposes `sanitization` 2.0's fill error without
    /// changing the return type of [`Self::decode_locked_secret_bytes`]. The
    /// provided [`base64_ng::ct::CtEngine`] implementation uses the
    /// integrity-checked fill API directly. The compatibility default for
    /// external trait implementations maps the older generation error; custom
    /// implementations that need integrity-aware initialization must override
    /// this method.
    ///
    /// For allocation-time fail-closed protection, use
    /// [`Self::decode_locked_secret_bytes_checked`].
    ///
    /// # Errors
    ///
    /// Returns a `sanitization` memory or integrity error when locked storage
    /// cannot be initialized. Decode and exact-length failures are returned in
    /// the fill error's generator branch.
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
    fn decode_locked_secret_bytes_fill<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretBytes<N>, LockedSecretBytesFillError<SanitizationDecodeError>> {
        self.decode_locked_secret_bytes(input)
            .map_err(|error| match error {
                LockedSecretBytesGenerateError::Memory(error) => {
                    LockedSecretBytesFillError::Memory(error)
                }
                LockedSecretBytesGenerateError::Generate(error) => {
                    LockedSecretBytesFillError::Generate(error)
                }
            })
    }

    /// Decode into fixed-size locked storage and reject degraded protection.
    ///
    /// In the provided [`base64_ng::ct::CtEngine`] implementation, this
    /// fail-closed helper requires memory locking, dump exclusion, and fork
    /// exclusion before the decoder receives the destination mapping. When
    /// canaries are enabled, they are required too. Plaintext is decoded
    /// directly into the already protected mapping, without an ordinary stack
    /// plaintext buffer.
    ///
    /// The compatibility default for external trait implementations performs
    /// post-construction report admission. Custom implementations that require
    /// the same pre-decode guarantee must override this method and establish
    /// the requested protections before decoding.
    ///
    /// # Errors
    ///
    /// Returns [`LockedDecodeError::DegradedProtection`] when a required
    /// control cannot be established before decoding or integrity validation
    /// fails. This intentionally preserves the existing public error shape.
    /// Returns [`LockedDecodeError::Operation`] for decode or exact-length
    /// failures. Use
    /// [`CtDecodeSanitizationProtectedExt::decode_locked_secret_bytes_checked_detailed`]
    /// when protection and integrity failures require distinct incident
    /// handling.
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
    fn decode_locked_secret_bytes_checked<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<
        LockedSecretBytes<N>,
        LockedDecodeError<LockedSecretBytesGenerateError<SanitizationDecodeError>>,
    > {
        let secret = self
            .decode_locked_secret_bytes(input)
            .map_err(LockedDecodeError::Operation)?;
        let degraded = secret.protection_report().is_degraded();
        locked::admit_locked(secret, degraded)
    }

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
    ///
    /// # Security
    ///
    /// Successful construction does not prove that preferred dump and fork
    /// controls were established. Inspect `protection_report()` or use
    /// [`Self::decode_locked_secret_vec_checked`].
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

    /// Decode into dynamic locked storage and reject degraded protection.
    ///
    /// # Errors
    ///
    /// In the built-in implementation, returns
    /// [`LockedDecodeError::Operation`] for decode or length failures and
    /// [`LockedDecodeError::DegradedProtection`] when a required control
    /// cannot be established or integrity validation fails. The compatibility
    /// default also reports construction failures through `Operation`. Use
    /// [`CtDecodeSanitizationProtectedExt::decode_locked_secret_vec_checked_detailed`]
    /// for distinct protection and integrity failures, or
    /// [`CtDecodeSanitizationProtectedExt::decode_locked_secret_vec_checked_bounded`]
    /// to enforce a decoded-capacity limit before allocation.
    ///
    /// # Security
    ///
    /// In the provided [`base64_ng::ct::CtEngine`] implementation, the
    /// `sanitization` 2.0.3 protected-capacity constructor requires memory
    /// locking, dump exclusion, and fork exclusion before its fill closure can
    /// run. Canaries are required when enabled. A required-control failure
    /// therefore returns before decoded plaintext can enter the mapping.
    ///
    /// The compatibility default for external trait implementations performs
    /// post-construction report admission. Custom implementations that require
    /// the same pre-decode guarantee must override this method and establish
    /// the requested protections before decoding.
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
    fn decode_locked_secret_vec_checked(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretVec, LockedDecodeError<LockedSecretVecFillError<DecodeError>>> {
        let secret = self
            .decode_locked_secret_vec(input)
            .map_err(LockedDecodeError::Operation)?;
        let degraded = secret.protection_report().is_degraded();
        locked::admit_locked(secret, degraded)
    }
}

#[cfg(test)]
mod tests;
