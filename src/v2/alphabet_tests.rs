use super::{
    alphabet::{ALPHABET_LEN, ValidatedAlphabet, ValidatedAlphabetError},
    secret,
};
use crate::{Alphabet, Standard, UrlSafe, decode_alphabet_byte};

extern crate std;
use std::string::ToString;

const STANDARD_TABLE: [u8; ALPHABET_LEN] =
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE_TABLE: [u8; ALPHABET_LEN] =
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const CUSTOM_TABLE: [u8; ALPHABET_LEN] =
    *b"/+9876543210zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA";

const CONST_CUSTOM: ValidatedAlphabet = match ValidatedAlphabet::new(CUSTOM_TABLE) {
    Ok(alphabet) => alphabet,
    Err(_) => panic!("test alphabet must be valid"),
};

struct ConstCustom;

impl Alphabet for ConstCustom {
    const ENCODE: [u8; ALPHABET_LEN] = CUSTOM_TABLE;

    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

struct DivergentLegacyAlphabet;

impl Alphabet for DivergentLegacyAlphabet {
    const ENCODE: [u8; ALPHABET_LEN] = CUSTOM_TABLE;

    fn encode(_: u8) -> u8 {
        b'!'
    }

    fn decode(_: u8) -> Option<u8> {
        Some(0)
    }
}

#[test]
fn validated_alphabet_is_exactly_one_owned_table() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<ValidatedAlphabet>();
    assert_eq!(core::mem::size_of::<ValidatedAlphabet>(), ALPHABET_LEN);
    assert_eq!(core::mem::align_of::<ValidatedAlphabet>(), 1);
    assert_eq!(CONST_CUSTOM.as_array(), &CUSTOM_TABLE);
}

#[test]
fn const_and_runtime_construction_have_identical_mapping() {
    let runtime_array = ValidatedAlphabet::try_from(CUSTOM_TABLE).unwrap();
    let runtime_slice = ValidatedAlphabet::try_from(&CUSTOM_TABLE[..]).unwrap();

    assert_eq!(runtime_array, CONST_CUSTOM);
    assert_eq!(runtime_slice, CONST_CUSTOM);
    for value in 0u8..=u8::MAX {
        let expected = if usize::from(value) < ALPHABET_LEN {
            Some(CUSTOM_TABLE[usize::from(value)])
        } else {
            None
        };
        assert_eq!(CONST_CUSTOM.encode_value(value), expected);
        assert_eq!(runtime_array.encode_value(value), expected);
    }
    for byte in 0u8..=u8::MAX {
        let expected = ConstCustom::decode(byte);
        assert_eq!(CONST_CUSTOM.decode_byte(byte), expected);
        assert_eq!(runtime_slice.decode_byte(byte), expected);
        assert_eq!(secret::decode_alphabet_byte(&runtime_slice, byte), expected);
    }
}

#[test]
fn built_in_and_custom_tables_map_all_64_values() {
    for (table, legacy_decode) in [
        (STANDARD_TABLE, Standard::decode as fn(u8) -> Option<u8>),
        (URL_SAFE_TABLE, UrlSafe::decode as fn(u8) -> Option<u8>),
        (CUSTOM_TABLE, ConstCustom::decode as fn(u8) -> Option<u8>),
    ] {
        let alphabet = ValidatedAlphabet::new(table).unwrap();
        for (index, byte) in table.iter().copied().enumerate() {
            let value = u8::try_from(index).unwrap();
            assert_eq!(alphabet.encode_value(value), Some(byte));
            assert_eq!(alphabet.decode_byte(byte), Some(value));
            assert_eq!(secret::decode_alphabet_byte(&alphabet, byte), Some(value));
            assert_eq!(legacy_decode(byte), Some(value));
        }
    }
}

#[test]
fn new_mapping_cannot_observe_legacy_executable_overrides() {
    let alphabet = ValidatedAlphabet::new(DivergentLegacyAlphabet::ENCODE).unwrap();

    assert_eq!(DivergentLegacyAlphabet::encode(63), b'!');
    assert_eq!(DivergentLegacyAlphabet::decode(b'A'), Some(0));
    assert_eq!(alphabet.encode_value(63), Some(b'A'));
    assert_eq!(alphabet.decode_byte(b'A'), Some(63));
    assert_eq!(secret::decode_alphabet_byte(&alphabet, b'A'), Some(63));
}

#[test]
fn every_duplicate_position_pair_is_rejected_exactly() {
    for first in 0..ALPHABET_LEN {
        for second in first + 1..ALPHABET_LEN {
            let mut table = STANDARD_TABLE;
            table[second] = table[first];
            assert_eq!(
                ValidatedAlphabet::new(table),
                Err(ValidatedAlphabetError::DuplicateByte {
                    first,
                    second,
                    byte: STANDARD_TABLE[first],
                })
            );
        }
    }
}

#[test]
fn every_forbidden_byte_is_rejected_at_every_position() {
    for index in 0..ALPHABET_LEN {
        for byte in 0u8..=u8::MAX {
            let expected = if !(0x21..=0x7e).contains(&byte) {
                Some(ValidatedAlphabetError::InvalidByte { index, byte })
            } else if byte == b'=' {
                Some(ValidatedAlphabetError::PaddingByte { index })
            } else {
                None
            };
            let Some(expected) = expected else {
                continue;
            };

            let mut table = STANDARD_TABLE;
            table[index] = byte;
            assert_eq!(ValidatedAlphabet::new(table), Err(expected));
        }
    }
}

#[test]
fn checked_slice_conversion_rejects_every_nearby_wrong_length() {
    let mut storage = [b'!'; ALPHABET_LEN + 1];
    storage[..ALPHABET_LEN].copy_from_slice(&STANDARD_TABLE);

    for len in 0..=storage.len() {
        let result = ValidatedAlphabet::try_from_slice(&storage[..len]);
        if len == ALPHABET_LEN {
            assert_eq!(result.unwrap().as_array(), &STANDARD_TABLE);
        } else {
            assert_eq!(
                result,
                Err(ValidatedAlphabetError::InvalidLength { actual: len })
            );
        }
    }
}

#[test]
fn validation_errors_have_complete_diagnostics() {
    assert_eq!(
        ValidatedAlphabetError::InvalidLength { actual: 63 }.to_string(),
        "base64 alphabet has length 63; expected 64"
    );
    assert_eq!(
        ValidatedAlphabetError::InvalidByte {
            index: 7,
            byte: b'\n',
        }
        .to_string(),
        "invalid base64 alphabet byte 0x0a at index 7"
    );
    assert_eq!(
        ValidatedAlphabetError::PaddingByte { index: 8 }.to_string(),
        "base64 alphabet contains padding byte at index 8"
    );
    assert_eq!(
        ValidatedAlphabetError::DuplicateByte {
            first: 1,
            second: 9,
            byte: b'B',
        }
        .to_string(),
        "base64 alphabet byte 0x42 is duplicated at indexes 1 and 9"
    );
}
