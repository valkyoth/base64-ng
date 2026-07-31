#![cfg(feature = "secrets")]

extern crate std;

use core::mem::{needs_drop, size_of};
use std::format;

use super::{
    STRICT_STANDARD_PADDED,
    secret::{ExposedSecret, SecretArray, SecretInput, SecretOutput},
};

#[test]
fn secret_wrappers_require_explicit_exposure_and_redact_formatting() {
    let input = SecretInput::new(b"classified");
    assert_eq!(input.expose_secret().as_bytes(), b"classified");
    assert_eq!(
        format!("{input:?}"),
        "SecretInput { bytes: \"<redacted>\", len: 10 }"
    );
    assert_eq!(format!("{input}"), "<redacted secret>");

    let exposed: ExposedSecret<'_> = input.expose_secret();
    assert_eq!(AsRef::<[u8]>::as_ref(&exposed), b"classified");
    assert_eq!(
        format!("{exposed:?}"),
        "ExposedSecret { bytes: \"<redacted>\", len: 10 }"
    );
    assert_eq!(format!("{exposed}"), "<redacted secret>");

    let mut encoded = [0u8; 16];
    let written = STRICT_STANDARD_PADDED
        .encode_into(input.expose_secret().as_bytes(), &mut encoded)
        .unwrap();
    assert_eq!(&encoded[..written], b"Y2xhc3NpZmllZA==");
}

#[test]
fn secret_output_wipes_tail_and_complete_range_on_drop() {
    let mut backing = [0xa5; 12];
    backing[..3].copy_from_slice(b"key");
    {
        let mut output = SecretOutput::from_initialized(&mut backing, 3).unwrap();
        assert_eq!(output.expose_secret().as_bytes(), b"key");
        assert_eq!(output.capacity(), 12);
        assert_eq!(
            format!("{output:?}"),
            "SecretOutput { bytes: \"<redacted>\", len: 3 }"
        );
        assert_eq!(format!("{output}"), "<redacted secret>");
        {
            let mut exposed = output.expose_secret_mut();
            assert_eq!(
                format!("{exposed:?}"),
                "ExposedSecretMut { bytes: \"<redacted>\", len: 3 }"
            );
            assert_eq!(format!("{exposed}"), "<redacted secret>");
            exposed.as_bytes_mut()[0] = b'K';
        }
        assert_eq!(output.expose_secret().as_bytes(), b"Key");
    }
    assert_eq!(backing, [0; 12]);
}

#[test]
fn secret_output_rejection_wipes_caller_storage() {
    let mut backing = [0xa5; 4];
    let error = SecretOutput::from_initialized(&mut backing, 5).unwrap_err();
    assert_eq!(error.length(), 5);
    assert_eq!(error.capacity(), 4);
    assert_eq!(backing, [0; 4]);
}

#[cfg(feature = "std")]
#[test]
fn secret_output_unwind_wipes_caller_storage() {
    let mut backing = [0xa5; 8];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _output = SecretOutput::from_initialized(&mut backing, 8).unwrap();
        panic!("reviewed cleanup test");
    }));
    assert!(result.is_err());
    assert_eq!(backing, [0; 8]);
}

#[test]
fn declassification_is_visible_and_transfers_cleanup_responsibility() {
    let mut backing = [0xa5; 8];
    backing[..3].copy_from_slice(b"key");
    let output = SecretOutput::from_initialized(&mut backing, 3).unwrap();
    let ordinary = output.declassify();
    assert_eq!(ordinary.as_bytes(), b"key");
    let (storage, len) = ordinary.into_parts();
    assert_eq!(len, 3);
    assert_eq!(storage, b"key\0\0\0\0\0");
}

#[test]
fn fixed_secret_storage_wipes_tail_and_redacts() {
    let mut bytes = [0xa5; 8];
    bytes[..3].copy_from_slice(b"key");
    let mut secret = SecretArray::from_array(bytes, 3).unwrap();
    assert!(needs_drop::<SecretArray<8>>());
    assert_eq!(size_of::<SecretArray<8>>(), 8 + size_of::<usize>());
    assert_eq!(secret.expose_secret().as_bytes(), b"key");
    assert_eq!(&secret.backing_for_test()[3..], &[0; 5]);
    assert_eq!(
        format!("{secret:?}"),
        "SecretArray { bytes: \"<redacted>\", len: 3, capacity: 8 }"
    );
    assert_eq!(format!("{secret}"), "<redacted secret array>");

    secret.clear();
    assert!(secret.is_empty());
    assert_eq!(secret.backing_for_test(), &[0; 8]);
}

#[test]
fn fixed_secret_declassification_returns_ordinary_value() {
    let secret = SecretArray::from_array(*b"keyxxxxx", 3).unwrap();
    let ordinary = secret.declassify();
    assert_eq!(ordinary.as_bytes(), b"key");
    let (bytes, len) = ordinary.into_parts();
    assert_eq!(len, 3);
    assert_eq!(bytes, *b"key\0\0\0\0\0");
}
