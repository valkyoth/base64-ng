use super::*;
use crate::decode_validation::observation;
use crate::{Base64, CodecBuilder, DecodeValidation, RuntimeSpec, ValidatedAlphabet};

#[test]
fn every_byte_is_classified_identically_to_literal_alphabet_positions() {
    for (family, alphabet) in [(Family::Standard, STANDARD), (Family::UrlSafe, URL_SAFE)] {
        for byte in 0u8..=255 {
            let expected = alphabet
                .iter()
                .position(|&symbol| symbol == byte)
                .map_or(INVALID, |value| u8::try_from(value).unwrap());
            assert_eq!(family.table()[usize::from(byte)], expected);
        }
    }
}

fn runtime(settings: CodecSettings) -> Base64<RuntimeSpec> {
    CodecBuilder::new(*settings.alphabet())
        .encode_padding(settings.encode_padding())
        .decode_padding(settings.decode_padding())
        .trailing_bits(settings.trailing_bits())
        .build()
        .unwrap()
}

#[test]
fn exactly_equivalent_runtime_settings_use_the_fast_validator() {
    for settings in [
        crate::STRICT_STANDARD_PADDED.settings(),
        crate::STRICT_STANDARD_UNPADDED.settings(),
        crate::STRICT_URL_SAFE_PADDED.settings(),
        crate::STRICT_URL_SAFE_UNPADDED.settings(),
    ] {
        let codec = runtime(settings);
        let fast = observation::fast_calls();
        let reference = observation::calls();
        let mut output = [0xa5; 6];
        assert_eq!(codec.decode_into(b"Zm9v", &mut output), Ok(3));
        assert_eq!(observation::fast_calls(), fast + 1);
        assert_eq!(observation::calls(), reference);
        assert_eq!(output, *b"foo\xa5\xa5\xa5");
        codec
            .decode_into_with_validation(b"Zm9v", &mut output, DecodeValidation::ScalarReference)
            .unwrap();
        assert_eq!(observation::fast_calls(), fast + 1);
        assert_eq!(observation::calls(), reference + 1);
    }
}

#[test]
fn custom_permutations_and_relaxed_settings_are_never_specialized() {
    for offset in 1..64 {
        let mut alphabet = *STANDARD;
        alphabet.rotate_left(offset);
        let codec = CodecBuilder::new(ValidatedAlphabet::new(alphabet).unwrap())
            .build()
            .unwrap();
        assert_eq!(Family::for_settings(codec.settings()), None);
        let before = observation::fast_calls();
        let reference = observation::calls();
        let mut encoded = [0; 4];
        codec.encode_into(b"foo", &mut encoded).unwrap();
        let mut output = [0; 3];
        codec.decode_into(&encoded, &mut output).unwrap();
        assert_eq!(&output, b"foo");
        assert_eq!(observation::fast_calls(), before);
        assert_eq!(observation::calls(), reference + 1);
    }
    for alphabet in [*STANDARD, *URL_SAFE] {
        for padding in [
            DecodePadding::RequireCanonical,
            DecodePadding::Forbid,
            DecodePadding::Indifferent,
        ] {
            for bits in [
                TrailingBits::RequireCanonical,
                TrailingBits::AllowNonCanonical,
            ] {
                let encode = if padding == DecodePadding::Forbid {
                    EncodePadding::Unpadded
                } else {
                    EncodePadding::Padded
                };
                let codec = CodecBuilder::from_table(alphabet)
                    .unwrap()
                    .encode_padding(encode)
                    .decode_padding(padding)
                    .trailing_bits(bits)
                    .build()
                    .unwrap();
                assert_eq!(
                    Family::for_settings(codec.settings()).is_some(),
                    padding != DecodePadding::Indifferent && bits == TrailingBits::RequireCanonical
                );
            }
        }
    }
}

#[test]
fn every_sextet_combination_has_correct_canonical_tail_bits() {
    for (family, alphabet) in [(Family::Standard, STANDARD), (Family::UrlSafe, URL_SAFE)] {
        for first in 0..64 {
            for second in 0..64 {
                let two = [alphabet[first], alphabet[second]];
                assert_eq!(
                    family.validated_len(&two, false),
                    (second % 16 == 0).then_some(1)
                );
                let padded = [two[0], two[1], b'=', b'='];
                assert_eq!(
                    family.validated_len(&padded, true),
                    (second % 16 == 0).then_some(1)
                );
                for (third, &symbol) in alphabet.iter().enumerate() {
                    let three = [two[0], two[1], symbol];
                    assert_eq!(
                        family.validated_len(&three, false),
                        (third % 4 == 0).then_some(2)
                    );
                    let padded = [three[0], three[1], three[2], b'='];
                    assert_eq!(
                        family.validated_len(&padded, true),
                        (third % 4 == 0).then_some(2)
                    );
                }
            }
        }
    }
}

#[test]
fn malformed_input_recovers_reference_diagnostics_before_capacity() {
    for settings in [
        crate::STRICT_STANDARD_PADDED.settings(),
        crate::STRICT_STANDARD_UNPADDED.settings(),
        crate::STRICT_URL_SAFE_PADDED.settings(),
        crate::STRICT_URL_SAFE_UNPADDED.settings(),
    ] {
        let codec = runtime(settings);
        let mut input = [b'A'; 132];
        for index in 0..input.len() {
            for byte in [b'=', b'!', b' ', 0x80, 0xff] {
                input[index] = byte;
                for capacity in [0, 1, 98, 99, 104] {
                    let mut output = [0xa5; 104];
                    let mut reference = output;
                    let expected = codec.decode_into_with_validation(
                        &input,
                        &mut reference[..capacity],
                        DecodeValidation::ScalarReference,
                    );
                    assert_eq!(codec.decode_into(&input, &mut output[..capacity]), expected);
                    assert_eq!(output, reference);
                }
            }
            input[index] = b'A';
        }
    }
}
