#![no_std]

use base64_ng::{
    BCRYPT, CRYPT, DecodedBuffer, EncodedBuffer, Engine, LineEnding, LineWrap, MIME, PEM,
    Profile, STANDARD, URL_SAFE_NO_PAD, checked_encoded_len, ct, decoded_capacity, decoded_len,
    encoded_len,
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
    if ct::STANDARD.decoded_len(b"aGVsbG8=") != Ok(5) {
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
    let ct_decoded = match ct::STANDARD.decode_in_place_clear_tail(&mut ct_decoded) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };

    ct_decoded == b"hi"
}

pub fn clear_tail_surfaces() -> bool {
    let mut encoded = [0xAA; 8];
    let encoded_len = match STANDARD.encode_slice_clear_tail(b"hi", &mut encoded) {
        Ok(written) => written,
        Err(_) => return false,
    };
    if encoded_len != 4 || &encoded[..encoded_len] != b"aGk=" || encoded[encoded_len..] != [0; 4]
    {
        return false;
    }

    let mut small_encode_output = [0xAA; 3];
    if STANDARD
        .encode_slice_clear_tail(b"hi", &mut small_encode_output)
        .is_ok()
        || small_encode_output != [0; 3]
    {
        return false;
    }

    let mut decoded = [0xAA; 5];
    let decoded_len = match STANDARD.decode_slice_clear_tail(b"aGk=", &mut decoded) {
        Ok(written) => written,
        Err(_) => return false,
    };
    if decoded_len != 2 || &decoded[..decoded_len] != b"hi" || decoded[decoded_len..] != [0; 3] {
        return false;
    }

    let mut invalid_decode_output = [0xAA; 5];
    if STANDARD
        .decode_slice_clear_tail(b"!!!!", &mut invalid_decode_output)
        .is_ok()
        || invalid_decode_output != [0; 5]
    {
        return false;
    }

    let mut ct_decoded = [0xAA; 5];
    let ct_decoded_len = match ct::STANDARD.decode_slice_clear_tail(b"aGk=", &mut ct_decoded) {
        Ok(written) => written,
        Err(_) => return false,
    };
    if ct_decoded_len != 2
        || &ct_decoded[..ct_decoded_len] != b"hi"
        || ct_decoded[ct_decoded_len..] != [0; 3]
    {
        return false;
    }

    let mut in_place = *b"aGk=";
    let in_place_len = match STANDARD.decode_in_place_clear_tail(&mut in_place) {
        Ok(decoded) => decoded.len(),
        Err(_) => return false,
    };
    in_place_len == 2 && &in_place[..in_place_len] == b"hi" && in_place[in_place_len..] == [0; 2]
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
    if STANDARD.ct_decoder().decoded_len(b"aGVsbG8=") != Ok(5) {
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

pub fn named_profile_surfaces() -> bool {
    if !MIME.is_wrapped()
        || MIME.line_wrap() != Some(LineWrap::MIME)
        || MIME.line_len() != Some(76)
        || MIME.line_ending() != Some(LineEnding::CrLf)
    {
        return false;
    }
    if !PEM.is_wrapped()
        || PEM.line_wrap() != Some(LineWrap::PEM)
        || PEM.line_len() != Some(64)
        || PEM.line_ending() != Some(LineEnding::Lf)
    {
        return false;
    }
    if BCRYPT.is_padded() || BCRYPT.is_wrapped() || CRYPT.is_padded() || CRYPT.is_wrapped() {
        return false;
    }

    let mime_encoded = match MIME.encode_buffer::<82>(&[0x5a; 58]) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    if !MIME.validate(mime_encoded.as_bytes()) || !mime_encoded.as_bytes().contains(&b'\r') {
        return false;
    }
    let mime_decoded = match MIME.decode_buffer::<58>(mime_encoded.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };
    if mime_decoded.as_bytes() != [0x5a; 58] {
        return false;
    }

    let bcrypt = match BCRYPT.encode_buffer::<4>(&[0xff, 0xff, 0xff]) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    if bcrypt.as_bytes() != b"9999" {
        return false;
    }
    let bcrypt_decoded = match BCRYPT.decode_buffer::<3>(bcrypt.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };
    if bcrypt_decoded.as_bytes() != [0xff, 0xff, 0xff] {
        return false;
    }

    let crypt = match CRYPT.encode_buffer::<4>(&[0xff, 0xff, 0xff]) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    let crypt_decoded = match CRYPT.decode_buffer::<3>(crypt.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };
    crypt_decoded.as_bytes() == [0xff, 0xff, 0xff]
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

pub fn native_interop_surfaces() -> bool {
    let encoded = match EncodedBuffer::<8>::try_from(b"hello") {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    if encoded.as_bytes() != b"aGVsbG8=" {
        return false;
    }

    let decoded = match DecodedBuffer::<5>::try_from(b"aGVsbG8=") {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };
    if decoded.as_bytes() != b"hello" {
        return false;
    }

    let parsed = match "aGVsbG8=".parse::<DecodedBuffer<5>>() {
        Ok(decoded) => decoded,
        Err(_) => return false,
    };
    if parsed.as_bytes() != b"hello" {
        return false;
    }

    DecodedBuffer::<5>::try_from(b"aGVsbG8").is_err()
        && "aGVsbG8".parse::<DecodedBuffer<5>>().is_err()
}

#[cfg(test)]
mod tests {
    use super::{
        CONST_HELLO, clear_tail_surfaces, ct_stack_decode, custom_profile_surfaces, fmt_surfaces,
        in_place_surfaces, legacy_stack_decode, length_and_stack_state_surfaces,
        named_profile_surfaces, native_interop_surfaces, stack_round_trip, url_safe_round_trip,
        validate_only_surfaces, wrapped_round_trip,
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
        assert!(named_profile_surfaces());
    }

    #[test]
    fn validation_and_in_place_surfaces_work() {
        assert!(validate_only_surfaces());
        assert!(in_place_surfaces());
        assert!(clear_tail_surfaces());
        assert!(ct_stack_decode());
        assert!(length_and_stack_state_surfaces());
        assert!(fmt_surfaces());
        assert!(native_interop_surfaces());
    }
}
