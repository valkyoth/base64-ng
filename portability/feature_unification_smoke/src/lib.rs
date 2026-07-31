#![no_std]

use ordinary_provider::{Engine, Standard};

pub const ORDINARY: Engine<Standard, true> = accelerated_provider::STANDARD;
pub const SECRET_UNIFIED: Engine<Standard, true> = secret_provider::STANDARD;
pub const CHECKED_UNIFIED: Engine<Standard, true> = checked_provider::STANDARD;

pub fn encode_with_unified_features(input: &[u8], output: &mut [u8]) -> usize {
    ORDINARY
        .encode_slice(input, output)
        .expect("fixed smoke output is large enough")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_to_one_ordinary_type() {
        let mut output = [0u8; 8];
        let written = encode_with_unified_features(b"hello", &mut output);
        assert_eq!(&output[..written], b"aGVsbG8=");
        assert_eq!(ORDINARY, SECRET_UNIFIED);
        assert_eq!(ORDINARY, CHECKED_UNIFIED);
    }
}
