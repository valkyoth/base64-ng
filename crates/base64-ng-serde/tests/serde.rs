#![allow(missing_docs)]

use base64_ng::{MIME, PEM};
use base64_ng_serde::{Base64Standard, Base64UrlSafeNoPad};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct StandardMessage {
    #[serde(with = "base64_ng_serde::standard")]
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct UrlMessage {
    #[serde(with = "base64_ng_serde::url_safe_no_pad")]
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct StandardNoPadMessage {
    #[serde(with = "base64_ng_serde::standard_no_pad")]
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct UrlSafeMessage {
    #[serde(with = "base64_ng_serde::url_safe")]
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MimeMessage {
    #[serde(with = "base64_ng_serde::mime")]
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct PemMessage {
    #[serde(with = "base64_ng_serde::pem")]
    payload: Vec<u8>,
}

#[test]
fn serializes_standard_field() {
    let message = StandardMessage {
        payload: b"hello".to_vec(),
    };

    assert_eq!(
        serde_json::to_string(&message).unwrap(),
        r#"{"payload":"aGVsbG8="}"#
    );
    assert_eq!(
        serde_json::from_str::<StandardMessage>(r#"{"payload":"aGVsbG8="}"#).unwrap(),
        message
    );
}

#[test]
fn serializes_url_safe_no_pad_field() {
    let message = UrlMessage {
        payload: vec![0xfb, 0xff],
    };

    assert_eq!(
        serde_json::to_string(&message).unwrap(),
        r#"{"payload":"-_8"}"#
    );
    assert_eq!(
        serde_json::from_str::<UrlMessage>(r#"{"payload":"-_8"}"#).unwrap(),
        message
    );
}

#[test]
fn serializes_additional_field_profiles() {
    let standard_no_pad = StandardNoPadMessage {
        payload: b"hello".to_vec(),
    };
    assert_eq!(
        serde_json::to_string(&standard_no_pad).unwrap(),
        r#"{"payload":"aGVsbG8"}"#
    );
    assert_eq!(
        serde_json::from_str::<StandardNoPadMessage>(r#"{"payload":"aGVsbG8"}"#).unwrap(),
        standard_no_pad
    );

    let url_safe = UrlSafeMessage {
        payload: vec![0xfb, 0xff],
    };
    assert_eq!(
        serde_json::to_string(&url_safe).unwrap(),
        r#"{"payload":"-_8="}"#
    );
    assert_eq!(
        serde_json::from_str::<UrlSafeMessage>(r#"{"payload":"-_8="}"#).unwrap(),
        url_safe
    );

    let wrapped = vec![b'a'; 58];
    let mime = MimeMessage {
        payload: wrapped.clone(),
    };
    let expected_mime_payload = MIME.encode_string(&wrapped).unwrap();
    assert_eq!(
        serde_json::to_string(&mime).unwrap(),
        format!(
            r#"{{"payload":{}}}"#,
            serde_json::to_string(&expected_mime_payload).unwrap()
        )
    );
    assert_eq!(
        serde_json::from_str::<MimeMessage>(&serde_json::to_string(&mime).unwrap()).unwrap(),
        mime
    );

    let pem = PemMessage { payload: wrapped };
    let expected_pem_payload = PEM.encode_string(&pem.payload).unwrap();
    assert_eq!(
        serde_json::to_string(&pem).unwrap(),
        format!(
            r#"{{"payload":{}}}"#,
            serde_json::to_string(&expected_pem_payload).unwrap()
        )
    );
    assert_eq!(
        serde_json::from_str::<PemMessage>(&serde_json::to_string(&pem).unwrap()).unwrap(),
        pem
    );
}

#[test]
fn wrapper_types_round_trip() {
    let standard = Base64Standard::new(b"hello".to_vec());
    let encoded = serde_json::to_string(&standard).unwrap();
    assert_eq!(encoded, r#""aGVsbG8=""#);
    let decoded: Base64Standard = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.as_bytes(), b"hello");

    let url = Base64UrlSafeNoPad::new(vec![0xfb, 0xff]);
    let encoded = serde_json::to_string(&url).unwrap();
    assert_eq!(encoded, r#""-_8""#);
    let decoded: Base64UrlSafeNoPad = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.as_bytes(), &[0xfb, 0xff]);
}

#[test]
fn wrapper_equality_uses_explicit_impls() {
    let left = Base64Standard::new(b"hello".to_vec());
    let right = Base64Standard::new(b"hello".to_vec());
    let different = Base64Standard::new(b"world".to_vec());
    assert_eq!(left, right);
    assert_ne!(left, different);

    let left = Base64UrlSafeNoPad::new(vec![0xfb, 0xff]);
    let right = Base64UrlSafeNoPad::new(vec![0xfb, 0xff]);
    let different = Base64UrlSafeNoPad::new(vec![0xfa, 0xff]);
    assert_eq!(left, right);
    assert_ne!(left, different);
}
