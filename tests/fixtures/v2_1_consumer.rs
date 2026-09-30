#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
use base64_ng::STRICT_URL_SAFE_UNPADDED;
use base64_ng::{OneShotError, STRICT_STANDARD_PADDED};

pub fn caller_buffers(input: &[u8], output: &mut [u8]) -> Result<usize, OneShotError> {
    STRICT_STANDARD_PADDED.decode_into(input, output)
}

#[test]
fn existing_caller_buffer_and_historical_calls() {
    let mut encoded = [0u8; 8];
    let len = STRICT_STANDARD_PADDED
        .encode_into(b"hello", &mut encoded)
        .unwrap();
    let mut decoded = [0u8; 5];
    assert_eq!(caller_buffers(&encoded[..len], &mut decoded).unwrap(), 5);
    assert_eq!(&decoded, b"hello");
    assert_eq!(
        base64_ng::STANDARD
            .decode_slice(&encoded[..len], &mut decoded)
            .unwrap(),
        5
    );
    let before = decoded;
    assert!(caller_buffers(b"!!!!", &mut decoded).is_err());
    assert_eq!(decoded, before);
    assert_eq!(STRICT_URL_SAFE_UNPADDED.decoded_len(b"-_8").unwrap(), 2);
}

#[test]
fn existing_incremental_and_in_place_calls() {
    let mut decoder = STRICT_STANDARD_PADDED.decoder();
    let mut output = [0u8; 3];
    let step = decoder.update(b"Zm9v", &mut output).unwrap();
    assert_eq!(step.progress().input_consumed(), 4);
    assert_eq!(step.progress().output_produced(), 3);
    assert_eq!(
        decoder.finish(&mut []).unwrap().status(),
        base64_ng::Status::Complete
    );
    assert_eq!(&output, b"foo");
    let mut buffer = *b"Zm9v";
    assert_eq!(
        STRICT_STANDARD_PADDED
            .decode_in_place(&mut buffer, 4)
            .unwrap(),
        3
    );
    assert_eq!(&buffer[..3], b"foo");
}

#[cfg(feature = "alloc")]
#[test]
fn existing_owned_and_append_calls() {
    let stored = base64_ng::Base64String::encode(STRICT_STANDARD_PADDED, b"foo").unwrap();
    assert_eq!(stored.as_str(), "Zm9v");
    assert_eq!(stored.decode().unwrap(), b"foo");
    let mut output = base64_ng::decode(b"Zm9v").unwrap();
    STRICT_STANDARD_PADDED
        .decode_append(b"YmFy", &mut output)
        .unwrap();
    assert_eq!(output, b"foobar");
    assert_eq!(base64_ng::encode(b"foo").unwrap(), "Zm9v");
}
