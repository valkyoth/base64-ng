#![allow(missing_docs)]

use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
use base64_ng_tokio::{
    decode_reader_to_writer, decode_reader_to_writer_limited, decode_to_vec,
    encode_reader_to_writer, encode_reader_to_writer_limited, encode_to_vec,
};

#[tokio::test]
async fn encodes_reader_to_writer() {
    let mut input = &b"hello"[..];
    let mut output = Vec::new();

    let written = encode_reader_to_writer(&STANDARD, &mut input, &mut output)
        .await
        .unwrap();

    assert_eq!(written, 8);
    assert_eq!(output, b"aGVsbG8=");
}

#[tokio::test]
async fn decodes_reader_to_writer() {
    let mut input = &b"aGVsbG8="[..];
    let mut output = Vec::new();

    let written = decode_reader_to_writer(&STANDARD, &mut input, &mut output)
        .await
        .unwrap();

    assert_eq!(written, 5);
    assert_eq!(output, b"hello");
}

#[tokio::test]
async fn decode_does_not_write_on_malformed_input() {
    let mut input = &b"aGVsbG8=$"[..];
    let mut output = b"untouched".to_vec();

    let error = decode_reader_to_writer(&STANDARD, &mut input, &mut output)
        .await
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(output, b"untouched");
}

#[tokio::test]
async fn limited_encode_reports_oversized_input_before_writing() {
    let mut input = &b"hello"[..];
    let mut output = b"untouched".to_vec();

    let error = encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 4)
        .await
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(output, b"untouched");
}

#[tokio::test]
async fn limited_decode_reports_oversized_input_before_writing() {
    let mut input = &b"aGVsbG8="[..];
    let mut output = b"untouched".to_vec();

    let error = decode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 7)
        .await
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(output, b"untouched");
}

#[tokio::test]
async fn limited_reader_helpers_round_trip_at_limit() {
    let mut input = &b"hello"[..];
    let mut encoded = Vec::new();

    let written = encode_reader_to_writer_limited(&STANDARD, &mut input, &mut encoded, 5)
        .await
        .unwrap();

    assert_eq!(written, 8);
    assert_eq!(encoded, b"aGVsbG8=");

    let mut encoded_input = &encoded[..];
    let mut decoded = Vec::new();
    let written = decode_reader_to_writer_limited(&STANDARD, &mut encoded_input, &mut decoded, 8)
        .await
        .unwrap();

    assert_eq!(written, 5);
    assert_eq!(decoded, b"hello");
}

#[tokio::test]
async fn vec_helpers_round_trip() {
    let encoded = encode_to_vec(&URL_SAFE_NO_PAD, [0xfb, 0xff]).unwrap();
    assert_eq!(encoded, b"-_8");

    let decoded = decode_to_vec(&URL_SAFE_NO_PAD, encoded).unwrap();
    assert_eq!(decoded, [0xfb, 0xff]);
}
