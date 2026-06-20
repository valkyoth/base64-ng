#![allow(missing_docs)]

use base64_ng::DecodeError;
use base64_ng_derive::Base64Secret;

#[derive(Base64Secret)]
struct ApiKey([u8; 5]);

#[test]
fn decodes_fixed_secret_newtype() {
    let key = ApiKey::from_base64(b"aGVsbG8=").unwrap();

    assert_eq!(key.as_bytes(), b"hello");
    assert_eq!(
        format!("{key:?}"),
        r#"ApiKey { bytes: "<redacted>", len: 5 }"#
    );
}

#[test]
fn encodes_fixed_secret_newtype() {
    let key = ApiKey::from(*b"hello");
    let encoded = key.encode_base64::<8>().unwrap();

    assert_eq!(encoded.as_str(), "aGVsbG8=");
}

#[test]
fn rejects_length_mismatch() {
    assert_eq!(
        ApiKey::from_base64(b"aGk=").unwrap_err(),
        DecodeError::InvalidLength
    );
}

#[test]
fn implements_standard_conversion_traits() {
    let from_str: ApiKey = "aGVsbG8=".parse().unwrap();
    let from_str_try = ApiKey::try_from("aGVsbG8=").unwrap();
    let from_bytes_try = ApiKey::try_from(b"aGVsbG8=".as_slice()).unwrap();

    assert!(from_str.constant_time_eq(&from_str_try));
    assert!(from_str.constant_time_eq(&from_bytes_try));
    assert_eq!(AsRef::<[u8]>::as_ref(&from_str), b"hello");
}
