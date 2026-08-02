#![allow(missing_docs)]

use base64_ng::{DecodedArray, secret::SecretArray};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BoundedStandard {
    #[serde(with = "base64_ng_serde::bounded::standard")]
    payload: DecodedArray<5>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BoundedMime {
    #[serde(with = "base64_ng_serde::bounded::mime")]
    payload: DecodedArray<96>,
}

#[derive(Deserialize, Serialize)]
struct SecretStandard {
    #[serde(with = "base64_ng_serde::secret::standard")]
    payload: SecretArray<8>,
}

#[derive(Debug, Deserialize)]
struct ExactSecret {
    #[serde(deserialize_with = "base64_ng_serde::secret::standard::deserialize_exact")]
    payload: SecretArray<6>,
}

#[test]
fn bounded_standard_round_trips_at_capacity() {
    let message = BoundedStandard {
        payload: DecodedArray::from_array(*b"hello", 5).unwrap(),
    };
    let encoded = serde_json::to_string(&message).unwrap();
    assert_eq!(encoded, r#"{"payload":"aGVsbG8="}"#);
    assert_eq!(
        serde_json::from_str::<BoundedStandard>(&encoded).unwrap(),
        message
    );
}

#[test]
fn bounded_profiles_reject_oversized_and_malformed_input() {
    let oversized = serde_json::from_str::<BoundedStandard>(r#"{"payload":"aGVsbG8h"}"#)
        .unwrap_err()
        .to_string();
    assert!(oversized.contains("configured limit"));

    let malformed = serde_json::from_str::<BoundedStandard>(r#"{"payload":"aGVs!G8="}"#)
        .unwrap_err()
        .to_string();
    assert!(malformed.contains("invalid base64 input"));
    assert!(!malformed.contains("0x"));
}

#[test]
fn bounded_mime_streams_wrapped_input_without_compaction() {
    let plain = [b'a'; 58];
    let message = BoundedMime {
        payload: DecodedArray::from_array(
            {
                let mut bytes = [0u8; 96];
                bytes[..plain.len()].copy_from_slice(&plain);
                bytes
            },
            plain.len(),
        )
        .unwrap(),
    };
    let encoded = serde_json::to_string(&message).unwrap();
    assert!(encoded.contains("\\r\\n"));
    assert_eq!(
        serde_json::from_str::<BoundedMime>(&encoded).unwrap(),
        message
    );

    let malformed = encoded.replace("\\r\\n", "\\n");
    assert!(
        serde_json::from_str::<BoundedMime>(&malformed)
            .unwrap_err()
            .to_string()
            .contains("body layout")
    );
}

#[test]
fn secret_adapter_returns_wiping_redacted_storage() {
    let decoded = serde_json::from_str::<SecretStandard>(r#"{"payload":"c2VjcmV0"}"#).unwrap();
    assert_eq!(decoded.payload.expose_secret().as_bytes(), b"secret");
    assert!(format!("{:?}", decoded.payload).contains("<redacted>"));
    assert_eq!(
        serde_json::to_string(&decoded).unwrap(),
        r#"{"payload":"c2VjcmV0"}"#
    );
}

#[test]
fn secret_errors_are_opaque_for_malformed_oversized_and_wrong_length() {
    for input in [
        r#"{"payload":"c2Vj!mV0"}"#,
        r#"{"payload":"QUJDREVGR0hJSg=="}"#,
        r#"{"payload":"Zm9v"}"#,
    ] {
        let error = serde_json::from_str::<ExactSecret>(input)
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid secret base64 input"));
        assert!(!error.contains('!'));
        assert!(!error.contains("index"));
    }
}

#[test]
fn secret_exact_accepts_only_the_declared_length() {
    let decoded = serde_json::from_str::<ExactSecret>(r#"{"payload":"c2VjcmV0"}"#).unwrap();
    assert_eq!(decoded.payload.expose_secret().as_bytes(), b"secret");
}
