#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional `subtle::ConstantTimeEq` integration for `base64-ng`.
//!
//! The core `base64-ng` package stays zero-runtime-dependency. This companion
//! crate exists for applications that already admit `subtle` and want a
//! reviewed comparison primitive at the protocol boundary.
//!
//! Length is treated as public. Mismatched lengths return
//! [`subtle::Choice::from(0)`] immediately. Use fixed-size protocol tokens when
//! length must not vary. When the length itself is secret, compare fixed-size
//! arrays or fixed-width protocol buffers directly with
//! [`subtle::ConstantTimeEq`] instead of this public-length helper.

use base64_ng::{DecodedBuffer, EncodedBuffer};
use subtle::{Choice, ConstantTimeEq};

#[cfg(feature = "alloc")]
use base64_ng::SecretBuffer;

/// Extension trait for comparing `base64-ng` buffers with `subtle`.
///
/// The comparison delegates equal-length byte comparisons to
/// [`subtle::ConstantTimeEq`]. Length mismatch remains public and returns
/// `Choice::from(0)`.
pub trait SubtleEqExt {
    /// Compares `self` with `expected` using `subtle` for equal-length inputs.
    ///
    /// Length is public. If lengths differ, this returns `Choice::from(0)`.
    #[must_use = "use Choice or convert it deliberately with bool::from(choice)"]
    fn subtle_ct_eq(&self, expected: &[u8]) -> Choice;

    /// Convenience boolean wrapper around [`Self::subtle_ct_eq`].
    ///
    /// Prefer [`Self::subtle_ct_eq`] when composing with other `subtle`
    /// decisions.
    #[must_use]
    fn subtle_verify(&self, expected: &[u8]) -> bool {
        bool::from(self.subtle_ct_eq(expected))
    }
}

impl SubtleEqExt for [u8] {
    fn subtle_ct_eq(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self, expected)
    }
}

impl SubtleEqExt for &[u8] {
    fn subtle_ct_eq(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self, expected)
    }
}

impl<const CAP: usize> SubtleEqExt for DecodedBuffer<CAP> {
    fn subtle_ct_eq(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.as_bytes(), expected)
    }
}

impl<const CAP: usize> SubtleEqExt for EncodedBuffer<CAP> {
    fn subtle_ct_eq(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.as_bytes(), expected)
    }
}

#[cfg(feature = "alloc")]
impl SubtleEqExt for SecretBuffer {
    fn subtle_ct_eq(&self, expected: &[u8]) -> Choice {
        subtle_ct_eq_public_len(self.expose_secret(), expected)
    }
}

/// Compares two byte slices with public length.
///
/// Equal-length comparisons are delegated to [`subtle::ConstantTimeEq`].
/// Mismatched lengths return `Choice::from(0)` immediately.
///
/// Use [`subtle::ConstantTimeEq`] directly on fixed-size arrays or fixed-width
/// protocol buffers when token length must not be observable.
#[must_use = "use Choice or convert it deliberately with bool::from(choice)"]
pub fn subtle_ct_eq_public_len(left: &[u8], right: &[u8]) -> Choice {
    if left.len() == right.len() {
        left.ct_eq(right)
    } else {
        Choice::from(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{SubtleEqExt, subtle_ct_eq_public_len};
    use base64_ng::STANDARD;

    #[cfg(feature = "alloc")]
    use base64_ng::ct;

    #[test]
    fn compares_raw_slices_with_public_length() {
        assert!(bool::from(subtle_ct_eq_public_len(b"hello", b"hello")));
        assert!(!bool::from(subtle_ct_eq_public_len(b"hello", b"world")));
        assert!(!bool::from(subtle_ct_eq_public_len(b"hello", b"hello!")));
    }

    #[test]
    fn compares_stack_backed_buffers() {
        let decoded = STANDARD.decode_buffer::<5>(b"aGVsbG8=").unwrap();
        assert!(decoded.subtle_verify(b"hello"));
        assert!(!decoded.subtle_verify(b"world"));

        let encoded = STANDARD.encode_buffer::<8>(b"hello").unwrap();
        assert!(encoded.subtle_verify(b"aGVsbG8="));
        assert!(!encoded.subtle_verify(b"aGVsbG8h"));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn compares_secret_buffer() {
        let decoded = ct::STANDARD.decode_secret(b"aGVsbG8=").unwrap();
        assert!(decoded.subtle_verify(b"hello"));
        assert!(!decoded.subtle_verify(b"world"));
    }
}
