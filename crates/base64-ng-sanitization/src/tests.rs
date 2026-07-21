use crate::{
    CtDecodeSanitizationExt, SanitizationCtEqExt, SanitizationDecodeError,
    sanitization_ct_eq_public_len,
};
use base64_ng::{DecodeError, ct};

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
use sanitization::{LockedSecretBytesFillError, LockedSecretBytesGenerateError};

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
use crate::{LockedDecodeError, LockedSanitizationCtEqExt};

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
use sanitization::LockedSecretVecFillError;

#[test]
fn decodes_fixed_secret_bytes() {
    let secret = ct::STANDARD.decode_secret_bytes::<5>(b"aGVsbG8=").unwrap();
    secret.expose_secret(|bytes| assert_eq!(bytes, b"hello"));
}

#[test]
fn compares_fixed_secret_bytes_with_native_ct_choice() {
    let secret = ct::STANDARD.decode_secret_bytes::<5>(b"aGVsbG8=").unwrap();
    assert!(secret.sanitization_verify(b"hello", "test declassifies equality result"));
    assert!(
        !secret
            .sanitization_ct_eq(b"world")
            .declassify("test declassifies inequality result")
    );
    assert!(
        !secret
            .sanitization_ct_eq(b"hello!")
            .declassify("test declassifies length mismatch")
    );
}

#[test]
fn compares_raw_slices_with_native_ct_choice() {
    assert!(
        sanitization_ct_eq_public_len(b"hello", b"hello")
            .declassify("test declassifies public-length equality")
    );
    assert!(
        !sanitization_ct_eq_public_len(b"hello", b"world")
            .declassify("test declassifies public-length inequality")
    );
}

#[test]
fn fixed_secret_bytes_reject_length_mismatch() {
    assert_eq!(
        ct::STANDARD
            .decode_secret_bytes::<4>(b"aGVsbG8=")
            .unwrap_err(),
        SanitizationDecodeError::LengthMismatch {
            expected: 4,
            actual: 5
        }
    );
}

#[test]
fn fixed_secret_bytes_reports_decode_error() {
    assert_eq!(
        ct::STANDARD
            .decode_secret_bytes::<5>(b"aGVsbG8!")
            .unwrap_err(),
        SanitizationDecodeError::Decode(DecodeError::InvalidInput)
    );
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
#[test]
fn decodes_fixed_secret_bytes_into_locked_memory() {
    let secret = match ct::STANDARD.decode_locked_secret_bytes::<5>(b"aGVsbG8=") {
        Ok(secret) => secret,
        Err(LockedSecretBytesGenerateError::Memory(_)) => return,
        Err(error) => panic!("unexpected locked fixed decode error: {error:?}"),
    };

    secret
        .try_expose_secret(|bytes| assert_eq!(bytes, b"hello"))
        .unwrap();
    assert!(
        secret
            .try_sanitization_verify(b"hello", "test declassifies locked fixed equality")
            .unwrap()
    );
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
#[test]
fn fill_decode_exposes_sanitization_2_integrity_errors() {
    let secret = match ct::STANDARD.decode_locked_secret_bytes_fill::<5>(b"aGVsbG8=") {
        Ok(secret) => secret,
        Err(LockedSecretBytesFillError::Memory(_)) => return,
        Err(error) => panic!("unexpected locked fixed fill error: {error:?}"),
    };

    secret
        .try_expose_secret(|bytes| assert_eq!(bytes, b"hello"))
        .unwrap();
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
#[test]
fn checked_locked_fixed_decode_rejects_degraded_protection() {
    match ct::STANDARD.decode_locked_secret_bytes_checked::<5>(b"aGVsbG8=") {
        Ok(secret) => assert!(!secret.protection_report().is_degraded()),
        Err(LockedDecodeError::DegradedProtection) => {}
        Err(error) => panic!("unexpected checked locked fixed decode error: {error:?}"),
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
#[test]
fn locked_fixed_secret_bytes_reject_length_mismatch() {
    assert!(matches!(
        ct::STANDARD.decode_locked_secret_bytes::<4>(b"aGVsbG8="),
        Err(LockedSecretBytesGenerateError::Generate(
            SanitizationDecodeError::LengthMismatch {
                expected: 4,
                actual: 5,
            },
        ))
    ));
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
#[test]
fn locked_fixed_secret_bytes_reports_decode_error() {
    assert!(matches!(
        ct::STANDARD.decode_locked_secret_bytes::<5>(b"aGVsbG8!"),
        Err(LockedSecretBytesGenerateError::Generate(
            SanitizationDecodeError::Decode(DecodeError::InvalidInput)
        ))
    ));
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
#[test]
fn checked_locked_fixed_decode_preserves_decode_errors() {
    match ct::STANDARD.decode_locked_secret_bytes_checked::<5>(b"aGVsbG8!") {
        Err(
            LockedDecodeError::Operation(LockedSecretBytesGenerateError::Generate(
                SanitizationDecodeError::Decode(DecodeError::InvalidInput),
            ))
            | LockedDecodeError::DegradedProtection,
        ) => {}
        result => panic!("unexpected checked locked decode result: {result:?}"),
    }
}

#[cfg(feature = "alloc")]
#[test]
fn decodes_secret_vec() {
    let secret = ct::STANDARD.decode_secret_vec(b"aGVsbG8=").unwrap();
    secret.with_secret(|bytes| assert_eq!(bytes, b"hello"));
}

#[cfg(feature = "alloc")]
#[test]
fn compares_secret_vec_with_native_ct_choice() {
    let secret = ct::STANDARD.decode_secret_vec(b"aGVsbG8=").unwrap();
    assert!(secret.sanitization_verify(b"hello", "test declassifies SecretVec equality"));
    assert!(
        !secret
            .sanitization_ct_eq(b"world")
            .declassify("test declassifies SecretVec inequality")
    );
}

#[cfg(feature = "alloc")]
#[test]
fn decodes_secret_vec_staged() {
    let secret = ct::STANDARD
        .decode_secret_vec_staged::<5>(b"aGVsbG8=")
        .unwrap();
    secret.with_secret(|bytes| assert_eq!(bytes, b"hello"));
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
#[test]
fn decodes_secret_vec_into_locked_memory() {
    let secret = match ct::STANDARD.decode_locked_secret_vec(b"aGVsbG8=") {
        Ok(secret) => secret,
        Err(LockedSecretVecFillError::Memory(_)) => return,
        Err(error) => panic!("unexpected locked vec decode error: {error:?}"),
    };

    secret
        .try_with_secret(|bytes| assert_eq!(bytes, b"hello"))
        .unwrap();
    assert!(
        secret
            .try_sanitization_verify(b"hello", "test declassifies locked vec equality")
            .unwrap()
    );
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
#[test]
fn checked_locked_vec_decode_rejects_degraded_protection() {
    match ct::STANDARD.decode_locked_secret_vec_checked(b"aGVsbG8=") {
        Ok(secret) => assert!(!secret.protection_report().is_degraded()),
        Err(
            LockedDecodeError::DegradedProtection
            | LockedDecodeError::Operation(LockedSecretVecFillError::Memory(_)),
        ) => {}
        Err(error) => panic!("unexpected checked locked vec decode error: {error:?}"),
    }
}
