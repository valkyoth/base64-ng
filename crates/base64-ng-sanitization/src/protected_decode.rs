use base64_ng::{Alphabet, ct::CtEngine};

#[cfg(any(
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
))]
use crate::{SanitizationDecodeError, locked};

#[cfg(any(
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
))]
use sanitization::{LockedSecretBytes, LockedSecretInitializeError, ProtectedSecretFillError};

#[cfg(all(
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
use base64_ng::DecodeError;

#[cfg(all(
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
use sanitization::LockedSecretVec;

/// Detailed fail-closed decode helpers for protected locked storage.
///
/// Unlike the compatibility methods on [`crate::CtDecodeSanitizationExt`],
/// these helpers preserve `sanitization`'s distinction between protection
/// setup failures and canary-integrity failures. This lets high-assurance
/// applications alert on integrity events separately from unavailable OS
/// controls.
pub trait CtDecodeSanitizationProtectedExt {
    /// Decode an exact-size secret after required controls are established.
    ///
    /// # Errors
    ///
    /// Returns [`ProtectedSecretFillError::Protection`] when required controls
    /// cannot be established, [`ProtectedSecretFillError::Integrity`] when
    /// canary validation fails, and [`ProtectedSecretFillError::Fill`] for
    /// decode or exact-length failures.
    #[cfg(any(
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
    ))]
    fn decode_locked_secret_bytes_checked_detailed<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretBytes<N>, ProtectedSecretFillError<SanitizationDecodeError>>;

    /// Decode a runtime-size secret after required controls are established.
    ///
    /// # Errors
    ///
    /// Preserves every [`ProtectedSecretFillError`] failure class, including
    /// distinct protection and integrity errors.
    #[cfg(all(
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
    fn decode_locked_secret_vec_checked_detailed(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretVec, ProtectedSecretFillError<DecodeError>>;

    /// Decode a runtime-size secret with a pre-allocation capacity limit.
    ///
    /// `MAX` is a public application limit on decoded bytes. Input whose exact
    /// decoded length exceeds `MAX` is rejected before mapping allocation,
    /// protection setup, or decoder invocation.
    ///
    /// # Errors
    ///
    /// Returns [`ProtectedSecretFillError::CapacityLimit`] when the decoded
    /// capacity exceeds `MAX`. Other variants retain their detailed protection,
    /// integrity, decode, and length meanings.
    #[cfg(all(
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
    fn decode_locked_secret_vec_checked_bounded<const MAX: usize>(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretVec, ProtectedSecretFillError<DecodeError>>;
}

impl<A, const PAD: bool> CtDecodeSanitizationProtectedExt for CtEngine<A, PAD>
where
    A: Alphabet,
{
    #[cfg(any(
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
    ))]
    fn decode_locked_secret_bytes_checked_detailed<const N: usize>(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretBytes<N>, ProtectedSecretFillError<SanitizationDecodeError>> {
        let required = self
            .decoded_len(input)
            .map_err(|error| ProtectedSecretFillError::Fill(error.into()))?;
        if required != N {
            return Err(ProtectedSecretFillError::Fill(
                SanitizationDecodeError::LengthMismatch {
                    expected: N,
                    actual: required,
                },
            ));
        }

        let secret =
            LockedSecretBytes::zeroed_with_protection(locked::required_secret_protection())
                .map_err(ProtectedSecretFillError::Protection)?;
        secret
            .try_init_with(|output| {
                let written = self.decode_slice_clear_tail(input, output)?;
                if written != N {
                    return Err(SanitizationDecodeError::LengthMismatch {
                        expected: N,
                        actual: written,
                    });
                }
                Ok(())
            })
            .map_err(|error| match error {
                LockedSecretInitializeError::Integrity(error) => {
                    ProtectedSecretFillError::Integrity(error)
                }
                LockedSecretInitializeError::Generate(error) => {
                    ProtectedSecretFillError::Fill(error)
                }
            })
    }

    #[cfg(all(
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
    fn decode_locked_secret_vec_checked_detailed(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretVec, ProtectedSecretFillError<DecodeError>> {
        let required = self
            .decoded_len(input)
            .map_err(ProtectedSecretFillError::Fill)?;
        LockedSecretVec::try_from_capacity_with_protection(
            required,
            locked::required_secret_protection(),
            |output| self.decode_slice_clear_tail(input, output),
        )
    }

    #[cfg(all(
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
    fn decode_locked_secret_vec_checked_bounded<const MAX: usize>(
        &self,
        input: &[u8],
    ) -> Result<LockedSecretVec, ProtectedSecretFillError<DecodeError>> {
        let required = self
            .decoded_len(input)
            .map_err(ProtectedSecretFillError::Fill)?;
        LockedSecretVec::try_from_capacity_bounded_with_protection(
            required,
            MAX,
            locked::required_secret_protection(),
            |output| self.decode_slice_clear_tail(input, output),
        )
    }
}
