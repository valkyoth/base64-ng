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

use crate::{
    CtDecodeSanitizationProtectedExt, ProtectedAllocation, SanitizationProtectedDecodeError,
    SanitizationProtectedDecodeExt,
};
use base64_ng::{
    DecodeError, STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED, ct, secret::SecretInput,
};
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
#[test]
fn bounded_dynamic_decode_rejects_oversized_input_before_validation() {
    assert!(matches!(
        ct::STANDARD.decode_locked_secret_vec_checked_bounded::<4>(b"!!!!!!!!!!!!"),
        Err(ProtectedSecretFillError::CapacityLimit {
            maximum: 4,
            actual: 9
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

#[test]
fn v2_fixed_decode_uses_protected_staging_and_destination() {
    let input = SecretInput::new(b"aGVsbG8=");
    let secret = match STRICT_STANDARD_PADDED.decode_sanitization_protected_bytes::<5>(&input) {
        Ok(secret) => secret,
        Err(SanitizationProtectedDecodeError::Protection { .. }) => return,
        Err(error) => panic!("unexpected protected fixed decode error: {error:?}"),
    };

    secret
        .try_expose_secret(|bytes| assert_eq!(bytes, b"hello"))
        .unwrap();
}

#[test]
fn v2_fixed_decode_rejects_invalid_and_wrong_length_frames() {
    let invalid = SecretInput::new(b"aGVsbG8!");
    match STRICT_STANDARD_PADDED.decode_sanitization_protected_bytes::<5>(&invalid) {
        Err(SanitizationProtectedDecodeError::Protection { .. }) => return,
        Err(SanitizationProtectedDecodeError::Decode(
            base64_ng::secret::SecretDecodeError::InvalidInput,
        )) => {}
        result => panic!("unexpected invalid-input result: {result:?}"),
    }

    let short = SecretInput::new(b"aGVsbG8=");
    assert!(matches!(
        STRICT_STANDARD_PADDED.decode_sanitization_protected_bytes::<6>(&short),
        Err(SanitizationProtectedDecodeError::LengthMismatch {
            expected: 6,
            actual: 5,
        })
    ));
}

#[test]
fn v2_bounded_dynamic_decode_retains_only_validated_prefix() {
    let input = SecretInput::new(b"aGVsbG8=");
    let secret = match STRICT_STANDARD_PADDED.decode_sanitization_protected_vec::<64>(&input) {
        Ok(secret) => secret,
        Err(SanitizationProtectedDecodeError::Protection { .. }) => return,
        Err(error) => panic!("unexpected protected dynamic decode error: {error:?}"),
    };

    assert_eq!(secret.len(), 5);
    secret
        .try_with_secret(|bytes| assert_eq!(bytes, b"hello"))
        .unwrap();
}

#[test]
fn v2_protected_decode_uses_the_selected_validated_specification() {
    let input = SecretInput::new(b"_-7d");
    let secret = match STRICT_URL_SAFE_UNPADDED.decode_sanitization_protected_bytes::<3>(&input) {
        Ok(secret) => secret,
        Err(SanitizationProtectedDecodeError::Protection { .. }) => return,
        Err(error) => panic!("unexpected protected URL-safe decode error: {error:?}"),
    };

    secret
        .try_expose_secret(|bytes| assert_eq!(bytes, &[0xff, 0xee, 0xdd]))
        .unwrap();
}

#[test]
fn v2_protected_errors_are_redacted() {
    let error = SanitizationProtectedDecodeError::Integrity {
        allocation: ProtectedAllocation::Destination,
    };
    let debug = format!("{error:?}");
    let display = error.to_string();

    assert!(!debug.contains("hello"));
    assert!(!display.contains("hello"));
    assert!(display.contains("integrity"));
}
