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

pub fn validate_only_surfaces() -> bool {
    STANDARD.validate(b"aGVsbG8=")
        && STANDARD.validate_legacy(b"aGVs\r\nbG8=")
        && !STANDARD.validate(b"aGVs\r\nbG8=")
        && URL_SAFE_NO_PAD.validate(b"-_8")
        && ct::STANDARD.validate(b"aGVsbG8=")
}

pub fn in_place_surfaces() -> bool {
    let mut encoded = [0u8; 8];
    encoded[..5].copy_from_slice(b"hello");
    let encoded = match STANDARD.encode_in_place(&mut encoded, 5) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    if encoded != b"aGVsbG8=" {
        return false;
    }

    let mut decoded = *b"aGVsbG8=";
    let decoded = match STANDARD.decode_in_place(&mut decoded) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };
    if decoded != b"hello" {
        return false;
    }

    let mut ct_decoded = *b"aGk=";
    let ct_decoded = match ct::STANDARD.decode_in_place(&mut ct_decoded) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    ct_decoded == b"hi"
}

pub fn legacy_stack_decode() -> bool {
    let decoded = match STANDARD.decode_buffer_legacy::<5>(b"aGVs\r\nbG8=") {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    decoded.as_bytes() == b"hello"
}

#[cfg(test)]
mod tests {
    use super::{
        CONST_HELLO, ct_stack_decode, in_place_surfaces, legacy_stack_decode, stack_round_trip,
        url_safe_round_trip, validate_only_surfaces, wrapped_round_trip,
    };

    #[test]
    fn const_encoding_matches_expected_output() {
        assert_eq!(CONST_HELLO, *b"aGVsbG8=");
    }

    #[test]
    fn stack_backed_surfaces_round_trip() {
        assert!(stack_round_trip(b"hello"));
        assert!(url_safe_round_trip(b"\xfb\xff"));
        assert!(wrapped_round_trip());
        assert!(legacy_stack_decode());
    }

    #[test]
    fn validation_and_in_place_surfaces_work() {
        assert!(validate_only_surfaces());
        assert!(in_place_surfaces());
        assert!(ct_stack_decode());
    }
}
