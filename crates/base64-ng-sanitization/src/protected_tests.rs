#![cfg(all(
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

use crate::CtDecodeSanitizationProtectedExt;
use base64_ng::{DecodeError, ct};
use sanitization::ProtectedSecretFillError;

#[test]
fn detailed_dynamic_decode_preserves_decode_failure_class() {
    assert!(matches!(
        ct::STANDARD.decode_locked_secret_vec_checked_detailed(b"aGVsbG8!"),
        Err(ProtectedSecretFillError::Fill(DecodeError::InvalidInput))
    ));
}

#[test]
fn bounded_dynamic_decode_rejects_capacity_before_protection_setup() {
    assert!(matches!(
        ct::STANDARD.decode_locked_secret_vec_checked_bounded::<4>(b"aGVsbG8="),
        Err(ProtectedSecretFillError::CapacityLimit {
            maximum: 4,
            actual: 5,
        })
    ));
}

#[test]
fn detailed_fixed_decode_preserves_decode_failure_class() {
    assert!(matches!(
        ct::STANDARD.decode_locked_secret_bytes_checked_detailed::<5>(b"aGVsbG8!"),
        Err(ProtectedSecretFillError::Fill(
            crate::SanitizationDecodeError::Decode(DecodeError::InvalidInput)
        ))
    ));
}
