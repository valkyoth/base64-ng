#![allow(missing_docs)]

use base64_ng::{DecodeError, EncodeError, STANDARD, URL_SAFE_NO_PAD};
use base64_ng_bytes::{BytesDecodeError, BytesEncodeError, EngineBytesExt};
use bytes::{Bytes, BytesMut};

#[test]
fn encodes_and_decodes_bytes() {
    let encoded = STANDARD.encode_bytes(b"hello").unwrap();
    assert_eq!(&encoded[..], b"aGVsbG8=");

    let decoded = STANDARD.decode_bytes(encoded).unwrap();
    assert_eq!(&decoded[..], b"hello");
}

#[test]
fn supports_buf_inputs() {
    let input = Bytes::from_static(b"\xfb\xff");
    let encoded = URL_SAFE_NO_PAD.encode_buf(input).unwrap();
    assert_eq!(&encoded[..], b"-_8");

    let decoded = URL_SAFE_NO_PAD.decode_buf(encoded).unwrap();
    assert_eq!(&decoded[..], &[0xfb, 0xff]);
}

#[test]
fn supports_limited_buf_inputs() {
    let encoded = STANDARD
        .encode_buf_limited(Bytes::from_static(b"hello"), 5)
        .unwrap();
    assert_eq!(&encoded[..], b"aGVsbG8=");

    let decoded = STANDARD.decode_buf_limited(encoded, 8).unwrap();
    assert_eq!(&decoded[..], b"hello");

    assert_eq!(
        STANDARD
            .encode_buf_limited(Bytes::from_static(b"hello"), 4)
            .unwrap_err(),
        BytesEncodeError::InputTooLarge {
            input_len: 5,
            max_input_len: 4
        }
    );
    assert_eq!(
        STANDARD
            .decode_buf_limited(Bytes::from_static(b"aGVsbG8="), 7)
            .unwrap_err(),
        BytesDecodeError::InputTooLarge {
            input_len: 8,
            max_input_len: 7
        }
    );
}

#[test]
fn writes_to_buf_mut() {
    let mut encoded = BytesMut::with_capacity(8);
    let written = STANDARD
        .encode_buf_to_mut(Bytes::from_static(b"hello"), &mut encoded)
        .unwrap();
    assert_eq!(written, 8);
    assert_eq!(&encoded[..], b"aGVsbG8=");

    let mut decoded = BytesMut::new();
    decoded.reserve(5);
    let written = STANDARD
        .decode_buf_to_mut(encoded.freeze(), &mut decoded)
        .unwrap();
    assert_eq!(written, 5);
    assert_eq!(&decoded[..], b"hello");
}

#[test]
fn writes_to_buf_mut_with_limits() {
    let mut encoded = BytesMut::with_capacity(8);
    let written = STANDARD
        .encode_buf_to_mut_limited(Bytes::from_static(b"hello"), &mut encoded, 5)
        .unwrap();
    assert_eq!(written, 8);
    assert_eq!(&encoded[..], b"aGVsbG8=");

    let mut decoded = BytesMut::new();
    decoded.reserve(5);
    let written = STANDARD
        .decode_buf_to_mut_limited(encoded.freeze(), &mut decoded, 8)
        .unwrap();
    assert_eq!(written, 5);
    assert_eq!(&decoded[..], b"hello");
}

#[test]
fn reports_small_outputs() {
    let mut small = [0u8; 4];
    let mut encoded = &mut small[..];
    assert_eq!(
        STANDARD
            .encode_buf_to_mut(Bytes::from_static(b"hello"), &mut encoded)
            .unwrap_err(),
        EncodeError::OutputTooSmall {
            required: 8,
            available: 4
        }
    );

    let mut small = [0u8; 4];
    let mut decoded = &mut small[..];
    assert_eq!(
        STANDARD
            .decode_buf_to_mut(Bytes::from_static(b"aGVsbG8="), &mut decoded)
            .unwrap_err(),
        DecodeError::OutputTooSmall {
            required: 5,
            available: 4
        }
    );
}

#[test]
fn limited_buf_to_mut_reports_input_limits_before_output_limits() {
    let mut small = [0u8; 4];
    let mut encoded = &mut small[..];
    assert_eq!(
        STANDARD
            .encode_buf_to_mut_limited(Bytes::from_static(b"hello"), &mut encoded, 4)
            .unwrap_err(),
        BytesEncodeError::InputTooLarge {
            input_len: 5,
            max_input_len: 4
        }
    );

    let mut small = [0u8; 4];
    let mut decoded = &mut small[..];
    assert_eq!(
        STANDARD
            .decode_buf_to_mut_limited(Bytes::from_static(b"aGVsbG8="), &mut decoded, 7)
            .unwrap_err(),
        BytesDecodeError::InputTooLarge {
            input_len: 8,
            max_input_len: 7
        }
    );
}
