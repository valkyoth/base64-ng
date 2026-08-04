#![allow(missing_docs)]

use base64_ng::{MIME_BODY_STRICT, STRICT_STANDARD_PADDED};
use base64_ng_serde::DEFAULT_SERDE_DECODE_MAX_LEN;
use serde::de::value::{BorrowedBytesDeserializer, Error};

fn borrowed(input: &[u8]) -> BorrowedBytesDeserializer<'_, Error> {
    BorrowedBytesDeserializer::new(input)
}

#[test]
fn compatibility_adapter_rejects_oversized_input_before_validation() {
    let maximum_encoded = STRICT_STANDARD_PADDED
        .encoded_len(DEFAULT_SERDE_DECODE_MAX_LEN)
        .unwrap();
    let input = vec![b'!'; maximum_encoded + 1];

    let error = base64_ng_serde::standard::deserialize(borrowed(&input)).unwrap_err();
    assert!(error.to_string().contains("configured limit"));
}

#[test]
fn caller_selected_limit_rejects_oversized_input_before_validation() {
    let input = [b'!'; 9];
    let error =
        base64_ng_serde::standard::deserialize_with_limit::<_, 4>(borrowed(&input)).unwrap_err();
    assert!(error.to_string().contains("configured limit"));
}

#[test]
fn bounded_adapter_rejects_oversized_input_before_validation() {
    let input = [b'!'; 9];
    let error =
        base64_ng_serde::bounded::standard::deserialize::<_, 5>(borrowed(&input)).unwrap_err();
    assert!(error.to_string().contains("configured limit"));
}

#[test]
fn wrapped_limits_run_before_body_layout_validation() {
    let maximum_payload = MIME_BODY_STRICT.codec().encoded_len(57).unwrap();
    let maximum_body = MIME_BODY_STRICT
        .wrapping()
        .checked_output_len(maximum_payload)
        .unwrap();
    let input = vec![b'!'; maximum_body + 1];

    let custom = base64_ng_serde::mime::deserialize_with_limit::<_, 57>(borrowed(&input))
        .unwrap_err()
        .to_string();
    assert!(custom.contains("configured limit"));
    assert!(!custom.contains("body layout"));

    let bounded = base64_ng_serde::bounded::mime::deserialize::<_, 57>(borrowed(&input))
        .unwrap_err()
        .to_string();
    assert!(bounded.contains("configured limit"));
    assert!(!bounded.contains("body layout"));
}
