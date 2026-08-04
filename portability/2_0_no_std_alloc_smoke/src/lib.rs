#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use base64_ng::{Base64String, OneShotError, StrictStandardPadded, prelude::*};

/// Encodes ordinary bytes into policy-carrying storage with `no_std + alloc`.
pub fn encode(input: &[u8]) -> Result<Base64String<StrictStandardPadded>, OneShotError> {
    Base64String::encode(STRICT_STANDARD_PADDED, input)
}

/// Validates ordinary encoded text with `no_std + alloc`.
pub fn parse(input: &str) -> Result<Base64String<StrictStandardPadded>, OneShotError> {
    Base64String::parse(STRICT_STANDARD_PADDED, input)
}

/// Decodes policy-carrying ordinary text with `no_std + alloc`.
pub fn decode(input: &Base64String<StrictStandardPadded>) -> Result<Vec<u8>, OneShotError> {
    input.decode()
}

/// Adopts an existing allocation without changing its codec policy.
pub fn adopt(
    input: String,
) -> Result<Base64String<StrictStandardPadded>, OneShotError> {
    Base64String::from_string(STRICT_STANDARD_PADDED, input)
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use alloc::string::String;

    #[test]
    fn policy_string_round_trips_with_only_alloc_enabled() {
        let encoded = super::encode(b"hello").unwrap();
        assert_eq!(encoded.as_str(), "aGVsbG8=");
        assert_eq!(super::decode(&encoded).unwrap(), b"hello");

        let parsed = super::parse("Zm9v").unwrap();
        assert_eq!(super::decode(&parsed).unwrap(), b"foo");

        let adopted = super::adopt(String::from("YmFy")).unwrap();
        assert_eq!(super::decode(&adopted).unwrap(), b"bar");
    }

    #[test]
    fn malformed_text_remains_rejected_without_std() {
        assert!(super::parse("!!!!").is_err());
        assert!(super::adopt(String::from("Zg=")).is_err());
    }
}
