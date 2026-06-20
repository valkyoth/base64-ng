#![allow(missing_docs)]

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
