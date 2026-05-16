#![no_std]

use base64_ng::{LineEnding, LineWrap, STANDARD, URL_SAFE_NO_PAD, ct};

pub const CONST_HELLO: [u8; 8] = STANDARD.encode_array(b"hello");

pub fn stack_round_trip(input: &[u8]) -> bool {
    let encoded = match STANDARD.encode_buffer::<88>(input) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    let decoded = match STANDARD.decode_buffer::<64>(encoded.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    decoded.as_bytes() == input
}

pub fn url_safe_round_trip(input: &[u8]) -> bool {
    let encoded = match URL_SAFE_NO_PAD.encode_buffer::<88>(input) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    let decoded = match URL_SAFE_NO_PAD.decode_buffer::<64>(encoded.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    decoded.as_bytes() == input
}

pub fn wrapped_round_trip() -> bool {
    let wrap = LineWrap::new(4, LineEnding::Lf);
    let encoded = match STANDARD.encode_wrapped_buffer::<9>(b"hello", wrap) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    let decoded = match STANDARD.decode_wrapped_buffer::<5>(encoded.as_bytes(), wrap) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    encoded.as_bytes() == b"aGVs\nbG8=" && decoded.as_bytes() == b"hello"
}

pub fn ct_stack_decode() -> bool {
    let decoded = match ct::STANDARD.decode_buffer::<5>(b"aGVsbG8=") {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    decoded.as_bytes() == b"hello"
}
