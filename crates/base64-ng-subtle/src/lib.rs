#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Reviewed `subtle::ConstantTimeEq` integration for 2.0 secret storage.
//!
//! The core `base64-ng` package stays zero-runtime-dependency. This companion
//! crate isolates the `subtle` dependency and exposes one sealed comparison
//! trait for the final 2.0 secret owners and views.
//!
//! [`SubtleSecretEq::subtle_ct_eq_public_len`] treats length as public. A
//! mismatch returns `subtle::Choice::from(0)` immediately. The method returns
//! [`Choice`] rather than an ordinary `bool`, so declassification remains
//! visible at the protocol boundary.

#[cfg(feature = "alloc")]
use base64_ng::secret::SecretVec;
use base64_ng::secret::{ExposedSecret, ExposedSecretMut, SecretArray, SecretInput, SecretOutput};
use subtle::{Choice, ConstantTimeEq};

mod sealed {
    #[cfg(feature = "alloc")]
    use base64_ng::secret::SecretVec;
    use base64_ng::secret::{
        ExposedSecret, ExposedSecretMut, SecretArray, SecretInput, SecretOutput,
    };

    pub trait Sealed {}

    impl<const CAP: usize> Sealed for SecretArray<CAP> {}
    impl Sealed for SecretInput<'_> {}
    impl Sealed for SecretOutput<'_> {}
    impl Sealed for ExposedSecret<'_> {}
    impl Sealed for ExposedSecretMut<'_> {}
    #[cfg(feature = "alloc")]
    impl Sealed for SecretVec {}
}

/// Sealed reviewed equality integration for `base64-ng` secret bytes.
///
/// Only crate-reviewed 2.0 secret owners and views implement this trait.
/// Length is public: mismatched lengths return `Choice::from(0)`
/// immediately. Equal-length inputs are delegated to
/// [`subtle::ConstantTimeEq`].
///
/// This trait deliberately provides no boolean convenience method. Convert or
/// compose the returned [`Choice`] explicitly at the protocol decision point.
///
/// # Example
///
/// ```
/// use base64_ng::{STRICT_STANDARD_PADDED, secret::{SecretArrayFrame, SecretInput}};
/// use base64_ng_subtle::SubtleSecretEq;
///
/// let mut frame = SecretArrayFrame::<5>::new(&STRICT_STANDARD_PADDED).unwrap();
/// frame.update(&SecretInput::new(b"aGVsbG8=")).unwrap();
/// let token = frame.finish().unwrap();
/// let accepted = bool::from(token.subtle_ct_eq_public_len(b"hello"));
/// assert!(accepted);
/// ```
pub trait SubtleSecretEq: sealed::Sealed {
    /// Compares the secret bytes with an expected public-length byte string.
    ///
    /// Length mismatch is a public early return. For a token, MAC, or key
    /// whose length must not vary, enforce the fixed width before this call.
    #[must_use = "compose Choice values or declassify explicitly at the protocol boundary"]
    fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice;
}

impl<const CAP: usize> SubtleSecretEq for SecretArray<CAP> {
    fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.expose_secret().as_bytes(), expected)
    }
}

impl SubtleSecretEq for SecretInput<'_> {
    fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.expose_secret().as_bytes(), expected)
    }
}

impl SubtleSecretEq for SecretOutput<'_> {
    fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.expose_secret().as_bytes(), expected)
    }
}

impl SubtleSecretEq for ExposedSecret<'_> {
    fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.as_bytes(), expected)
    }
}

impl SubtleSecretEq for ExposedSecretMut<'_> {
    fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.as_bytes(), expected)
    }
}

#[cfg(feature = "alloc")]
impl SubtleSecretEq for SecretVec {
    fn subtle_ct_eq_public_len(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.expose_secret().as_bytes(), expected)
    }
}

/// Compares byte slices while treating their lengths as public.
///
/// Equal-length comparisons are delegated to [`subtle::ConstantTimeEq`]. A
/// mismatch returns `Choice::from(0)` immediately. Prefer
/// [`SubtleSecretEq`] for `base64-ng` secret storage so the reviewed call site
/// is visible in source.
#[inline(never)]
#[must_use = "compose Choice values or declassify explicitly at the protocol boundary"]
pub fn subtle_ct_eq_public_len(left: &[u8], right: &[u8]) -> Choice {
    if left.len() == right.len() {
        left.ct_eq(right)
    } else {
        Choice::from(0)
    }
}

/// Compares two fixed-width byte arrays without a runtime length branch.
///
/// The width is a public compile-time fact. Secret owners with a variable
/// initialized length should use [`SubtleSecretEq::subtle_ct_eq_public_len`]
/// after their protocol has enforced the required width.
///
/// # Example
///
/// ```
/// use base64_ng_subtle::subtle_ct_eq_fixed_width;
///
/// assert!(bool::from(subtle_ct_eq_fixed_width(b"token", b"token")));
/// ```
#[must_use = "compose Choice values or declassify explicitly at the protocol boundary"]
pub fn subtle_ct_eq_fixed_width<const N: usize>(left: &[u8; N], right: &[u8; N]) -> Choice {
    left.ct_eq(right)
}
