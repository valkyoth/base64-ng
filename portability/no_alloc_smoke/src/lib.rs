#![no_std]

use base64_ng::{
    DecodedBuffer, EncodedBuffer, Engine, LineEnding, LineWrap, Profile, STANDARD,
    URL_SAFE_NO_PAD, checked_encoded_len, decoded_capacity, decoded_len, encoded_len, ct,
};
use core::fmt::Write as _;

base64_ng::define_alphabet! {
    struct SmokeAlphabet = b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
}

const SMOKE_NO_PAD: Engine<SmokeAlphabet, false> = Engine::new();

pub const CONST_HELLO: [u8; 8] = STANDARD.encode_array(b"hello");

struct FixedText<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> FixedText<CAP> {
    const fn new() -> Self {
        Self {
            bytes: [0u8; CAP],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl<const CAP: usize> core::fmt::Write for FixedText<CAP> {
    fn write_str(&mut self, input: &str) -> core::fmt::Result {
        let available = CAP - self.len;
        if input.len() > available {
            return Err(core::fmt::Error);
        }

        self.bytes[self.len..self.len + input.len()].copy_from_slice(input.as_bytes());
        self.len += input.len();
        Ok(())
    }
}

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
    let mut ct_policy = FixedText::<18>::new();
    if write!(&mut ct_policy, "{}", ct::STANDARD).is_err() {
        return false;
    }
    if ct_policy.as_bytes() != b"ct padded=true" {
        return false;
    }

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

pub fn custom_profile_surfaces() -> bool {
    let standard_profile = STANDARD.profile();
    if standard_profile.engine() != STANDARD
        || !standard_profile.is_padded()
        || standard_profile.is_wrapped()
    {
        return false;
    }

    let encoded = match SMOKE_NO_PAD.encode_buffer::<4>(&[0xff, 0xff, 0xff]) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    if encoded.as_bytes() != b"9999" {
        return false;
    }

    let decoded = match SMOKE_NO_PAD.decode_buffer::<3>(encoded.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };
    if decoded.as_bytes() != [0xff, 0xff, 0xff] {
        return false;
    }

    let wrap = match LineWrap::checked_new(4, LineEnding::Lf) {
        Some(wrap) => wrap,
        None => return false,
    };
    if wrap.line_len() != 4
        || wrap.line_ending() != LineEnding::Lf
        || wrap.line_ending().name() != "LF"
        || wrap.line_ending().as_str() != "\n"
        || wrap.line_ending().as_bytes() != b"\n"
    {
        return false;
    }
    let mut line_ending_name = FixedText::<4>::new();
    if write!(&mut line_ending_name, "{}", wrap.line_ending()).is_err() {
        return false;
    }
    if line_ending_name.as_bytes() != b"LF" {
        return false;
    }
    let mut wrap_name = FixedText::<8>::new();
    if write!(&mut wrap_name, "{}", wrap).is_err() {
        return false;
    }
    if wrap_name.as_bytes() != b"4:LF" {
        return false;
    }
    let profile = match Profile::checked_new(STANDARD, Some(wrap)) {
        Some(profile) => profile,
        None => return false,
    };

    if !profile.is_valid() || !profile.is_padded() || profile.engine() != STANDARD {
        return false;
    }
    if !profile.is_wrapped()
        || profile.line_len() != Some(4)
        || profile.line_ending() != Some(LineEnding::Lf)
    {
        return false;
    }
    let mut profile_policy = FixedText::<32>::new();
    if write!(&mut profile_policy, "{}", profile).is_err() {
        return false;
    }
    if profile_policy.as_bytes() != b"padded=true wrap=4:LF" {
        return false;
    }

    let wrapped = match profile.encode_buffer::<9>(b"hello") {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    wrapped.as_bytes() == b"aGVs\nbG8="
}

pub fn length_and_stack_state_surfaces() -> bool {
    if checked_encoded_len(5, true) != Some(8) {
        return false;
    }
    let mut engine_policy = FixedText::<16>::new();
    if write!(&mut engine_policy, "{}", STANDARD).is_err() {
        return false;
    }
    if engine_policy.as_bytes() != b"padded=true" {
        return false;
    }
    if encoded_len(5, true) != Ok(8) {
        return false;
    }
    if decoded_capacity(8) != 6 {
        return false;
    }
    if decoded_len(b"aGVsbG8=", true) != Ok(5) {
        return false;
    }

    let empty_encoded = EncodedBuffer::<8>::new();
    if !empty_encoded.is_empty()
        || empty_encoded.is_full()
        || empty_encoded.capacity() != 8
        || empty_encoded.remaining_capacity() != 8
    {
        return false;
    }

    let encoded = match STANDARD.encode_buffer::<8>(b"hello") {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    if encoded.is_empty()
        || !encoded.is_full()
        || encoded.remaining_capacity() != 0
        || encoded.as_str() != "aGVsbG8="
        || encoded.as_utf8() != Ok("aGVsbG8=")
    {
        return false;
    }

    let empty_decoded = DecodedBuffer::<5>::new();
    if !empty_decoded.is_empty()
        || empty_decoded.is_full()
        || empty_decoded.capacity() != 5
        || empty_decoded.remaining_capacity() != 5
    {
        return false;
    }

    let decoded = match STANDARD.decode_buffer::<5>(encoded.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    !decoded.is_empty()
        && decoded.is_full()
        && decoded.remaining_capacity() == 0
        && decoded.as_bytes() == b"hello"
}

pub fn fmt_surfaces() -> bool {
    let encoded = match STANDARD.encode_buffer::<8>(b"hello") {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };

    let mut output = FixedText::<8>::new();
    if write!(&mut output, "{encoded}").is_err() {
        return false;
    }

    output.as_bytes() == b"aGVsbG8="
}

#[cfg(test)]
mod tests {
    use super::{
        CONST_HELLO, ct_stack_decode, custom_profile_surfaces, in_place_surfaces,
        fmt_surfaces, legacy_stack_decode, length_and_stack_state_surfaces, stack_round_trip,
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
        assert!(custom_profile_surfaces());
    }

    #[test]
    fn validation_and_in_place_surfaces_work() {
        assert!(validate_only_surfaces());
        assert!(in_place_surfaces());
        assert!(ct_stack_decode());
        assert!(length_and_stack_state_surfaces());
        assert!(fmt_surfaces());
    }
}
