use sanitization::{
    CanaryCorruptedError, SecretBytes,
    ct::{self, Choice, ConstantTimeEq},
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
use sanitization::LockedSecretBytes;

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
use sanitization::LockedSecretVec;

/// Native `sanitization::ct` comparison helpers for decoded secret containers.
///
/// Length is public: mismatched lengths return [`Choice::FALSE`] immediately.
/// Use fixed-size protocol tokens when length must not vary. Converting
/// [`Choice`] to `bool` is declassification and requires an explicit reason.
/// For locked containers, this compatibility trait uses sanitization's
/// explicit panic-on-integrity-failure implementation because [`Choice`]
/// cannot carry a canary error. Prefer [`LockedSanitizationCtEqExt`] when the
/// application must propagate integrity failures.
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

/// Integrity-checked comparison helpers for locked secret containers.
///
/// `sanitization` 2.0 makes mapped-storage integrity checks fallible. Prefer
/// this trait over [`SanitizationCtEqExt`] for locked values when canary
/// corruption must be returned instead of triggering the underlying
/// compatibility trait's explicit fail-stop behavior.
pub trait LockedSanitizationCtEqExt {
    /// Compare this locked secret with `expected` after checking mapping
    /// integrity before and after access.
    ///
    /// # Errors
    ///
    /// Returns [`CanaryCorruptedError`] when mapped-storage integrity fails.
    fn try_sanitization_ct_eq(&self, expected: &[u8]) -> Result<Choice, CanaryCorruptedError>;

    /// Declassify an integrity-checked comparison with an audit reason.
    ///
    /// # Errors
    ///
    /// Returns [`CanaryCorruptedError`] when mapped-storage integrity fails.
    fn try_sanitization_verify(
        &self,
        expected: &[u8],
        reason: &'static str,
    ) -> Result<bool, CanaryCorruptedError> {
        self.try_sanitization_ct_eq(expected)
            .map(|choice| choice.declassify(reason))
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
        all(target_arch = "wasm32", feature = "wasm-compat"),
    )
))]
impl<const N: usize> LockedSanitizationCtEqExt for LockedSecretBytes<N> {
    fn try_sanitization_ct_eq(&self, expected: &[u8]) -> Result<Choice, CanaryCorruptedError> {
        self.try_expose_secret(|secret| ct::eq_public_len(secret, expected))
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
impl LockedSanitizationCtEqExt for LockedSecretVec {
    fn try_sanitization_ct_eq(&self, expected: &[u8]) -> Result<Choice, CanaryCorruptedError> {
        self.try_with_secret(|secret| ct::eq_public_len(secret, expected))
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
