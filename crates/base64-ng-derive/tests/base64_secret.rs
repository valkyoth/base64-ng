#![allow(missing_docs)]

use base64_ng::secret::{SecretDecodeError, SecretInput};
use base64_ng_derive::Base64Secret;

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "standard",
    padding = "padded",
    exact_length = 2,
    exposure = "none"
)]
struct StandardPaddedNone(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "standard",
    padding = "padded",
    exact_length = 2,
    exposure = "read"
)]
struct StandardPaddedRead(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "standard",
    padding = "padded",
    exact_length = 2,
    exposure = "read_write"
)]
struct StandardPaddedWrite(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "standard",
    padding = "unpadded",
    exact_length = 2,
    exposure = "none"
)]
struct StandardUnpaddedNone(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "standard",
    padding = "unpadded",
    exact_length = 2,
    exposure = "read"
)]
struct StandardUnpaddedRead(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "standard",
    padding = "unpadded",
    exact_length = 2,
    exposure = "read_write"
)]
struct StandardUnpaddedWrite(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "url_safe",
    padding = "padded",
    exact_length = 2,
    exposure = "none"
)]
struct UrlSafePaddedNone(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "url_safe",
    padding = "padded",
    exact_length = 2,
    exposure = "read"
)]
struct UrlSafePaddedRead(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "url_safe",
    padding = "padded",
    exact_length = 2,
    exposure = "read_write"
)]
struct UrlSafePaddedWrite(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "url_safe",
    padding = "unpadded",
    exact_length = 2,
    exposure = "none"
)]
struct UrlSafeUnpaddedNone(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "url_safe",
    padding = "unpadded",
    exact_length = 2,
    exposure = "read"
)]
struct UrlSafeUnpaddedRead(base64_ng::secret::SecretArray<2>);

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "url_safe",
    padding = "unpadded",
    exact_length = 2,
    exposure = "read_write"
)]
struct UrlSafeUnpaddedWrite(base64_ng::secret::SecretArray<2>);

fn input(bytes: &[u8]) -> SecretInput<'_> {
    SecretInput::new(bytes)
}

macro_rules! assert_round_trip {
    ($type:ty, $encoded:expr) => {{
        let secret = <$type>::decode_base64(&input($encoded)).unwrap();
        assert_eq!(<$type>::EXACT_LENGTH, 2);
        let encoded = secret.encode_base64().unwrap();
        assert_eq!(encoded.expose_secret().as_bytes(), $encoded);
        assert!(core::mem::needs_drop::<$type>());
    }};
}

#[test]
fn every_policy_combination_decodes_and_encodes() {
    assert_round_trip!(StandardPaddedNone, b"++8=");
    assert_round_trip!(StandardPaddedRead, b"++8=");
    assert_round_trip!(StandardPaddedWrite, b"++8=");
    assert_round_trip!(StandardUnpaddedNone, b"++8");
    assert_round_trip!(StandardUnpaddedRead, b"++8");
    assert_round_trip!(StandardUnpaddedWrite, b"++8");
    assert_round_trip!(UrlSafePaddedNone, b"--8=");
    assert_round_trip!(UrlSafePaddedRead, b"--8=");
    assert_round_trip!(UrlSafePaddedWrite, b"--8=");
    assert_round_trip!(UrlSafeUnpaddedNone, b"--8");
    assert_round_trip!(UrlSafeUnpaddedRead, b"--8");
    assert_round_trip!(UrlSafeUnpaddedWrite, b"--8");
}

#[test]
fn exposure_policy_generates_only_named_views() {
    let read = StandardPaddedRead::decode_base64(&input(b"++8=")).unwrap();
    assert_eq!(read.expose_secret().as_bytes(), &[0xfb, 0xef]);

    let mut read_write = UrlSafeUnpaddedWrite::decode_base64(&input(b"--8")).unwrap();
    read_write.expose_secret_mut().as_bytes_mut()[0] = 0;
    assert_eq!(read_write.expose_secret().as_bytes(), &[0, 0xef]);
}

#[test]
fn formatting_is_redacted() {
    let secret = StandardPaddedRead::decode_base64(&input(b"++8=")).unwrap();
    assert_eq!(
        format!("{secret:?}"),
        r#"StandardPaddedRead { secret: "<redacted>", len: 2 }"#
    );
    assert_eq!(format!("{secret}"), "<redacted secret>");
}

#[test]
fn exact_length_and_codec_policy_fail_closed() {
    assert_eq!(
        StandardPaddedNone::decode_base64(&input(b"Zg==")).unwrap_err(),
        SecretDecodeError::InvalidInput
    );
    assert_eq!(
        StandardPaddedNone::decode_base64(&input(b"--8=")).unwrap_err(),
        SecretDecodeError::InvalidInput
    );
    assert_eq!(
        UrlSafeUnpaddedNone::decode_base64(&input(b"--8=")).unwrap_err(),
        SecretDecodeError::InputTooLarge {
            input_len: 4,
            maximum_encoded_len: 3,
        }
    );
}
