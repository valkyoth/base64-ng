use crate::{CtDecodeSanitizationExt, SanitizationDecodeError};
use base64_ng::{Alphabet, ct::CtEngine};
use sanitization::{SecretBytes, SecureSanitize};

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
use crate::{LockedDecodeError, locked};

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
use sanitization::{
    LockedSecretBytes, LockedSecretBytesFillError, LockedSecretBytesGenerateError,
    LockedSecretInitializeError,
};

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
type CheckedLockedFixedResult<T> =
    Result<T, LockedDecodeError<LockedSecretBytesGenerateError<SanitizationDecodeError>>>;

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
pub(crate) fn validate_before_locked_fixed_allocation<A, const PAD: bool, const N: usize, T, F>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
    allocate_and_decode: F,
) -> CheckedLockedFixedResult<T>
where
    A: Alphabet,
    F: FnOnce() -> CheckedLockedFixedResult<T>,
{
    let required = engine.decoded_len(input).map_err(|error| {
        LockedDecodeError::Operation(LockedSecretBytesGenerateError::Generate(error.into()))
    })?;
    if required != N {
        return Err(LockedDecodeError::Operation(
            LockedSecretBytesGenerateError::Generate(SanitizationDecodeError::LengthMismatch {
                expected: N,
                actual: required,
            }),
        ));
    }

    allocate_and_decode()
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

        if written != N {
            output.secure_sanitize();
            return Err(LockedSecretBytesGenerateError::Generate(
                SanitizationDecodeError::LengthMismatch {
                    expected: N,
                    actual: written,
                },
            ));
        }

        let result = LockedSecretBytes::try_from_fn(|index| {
            Ok::<u8, SanitizationDecodeError>(output[index])
        });
        output.secure_sanitize();
        result
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
    fn decode_locked_secret_bytes_fill<const N: usize>(
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
        validate_before_locked_fixed_allocation::<A, PAD, N, _, _>(self, input, || {
            let secret =
                LockedSecretBytes::zeroed_with_protection(locked::required_secret_protection())
                    .map_err(|_| LockedDecodeError::DegradedProtection)?;

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
                    LockedSecretInitializeError::Integrity(_) => {
                        LockedDecodeError::DegradedProtection
                    }
                    LockedSecretInitializeError::Generate(error) => LockedDecodeError::Operation(
                        LockedSecretBytesGenerateError::Generate(error),
                    ),
                })
        })
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
